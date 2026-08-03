// Copyright 2026 Mark AB (markik)
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::collections::{BTreeMap, BTreeSet};

use chartulary::FacetId;
use graphshell::personal_sync::{
    PersonalGraphEvent, SyncProjection, SyncSelection as PersonalSyncSelection,
};
use mere::kernel::graph::{EdgeAssertion, Graph};
use muniment::Backend;
use serde::Serialize;
use serde_json::Value;
use thiserror::Error;
use uuid::Uuid;

use crate::host::{CONTEXT_FACET, READING_FACET};
use crate::{CleromancyHost, ContextSnapshot, HostError, Reading};

pub const SYNC_BATCH_SCHEMA: &str = "cleromancy.sync-batch/v1";

/// The explicit local setting controlling which Cleromancy truth may enter
/// Graphshell's personal graph. Reading sync includes its contexts because a
/// receipt without the context it binds cannot be independently understood.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CleromancySyncSelection {
    #[default]
    Off,
    Contexts,
    ContextsAndReadings,
}

impl CleromancySyncSelection {
    pub fn includes_contexts(self) -> bool {
        !matches!(self, Self::Off)
    }

    pub fn includes_readings(self) -> bool {
        matches!(self, Self::ContextsAndReadings)
    }

    /// Configure H7 to materialize only the named facets Cleromancy selected.
    pub fn personal_graph_selection(self) -> PersonalSyncSelection {
        let mut facets = Vec::new();
        if self.includes_contexts() {
            facets.push(CONTEXT_FACET);
        }
        if self.includes_readings() {
            facets.push(READING_FACET);
        }
        PersonalSyncSelection::default().with_facets(facets)
    }
}

/// A deterministic, bounded set of H7 events ready for an admitted personal
/// replica or resident `PersonalSyncHost` to author.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CleromancySyncBatch {
    pub schema: &'static str,
    pub selection: CleromancySyncSelection,
    pub events: Vec<PersonalGraphEvent>,
    pub contexts: usize,
    pub readings: usize,
    pub digest: String,
}

impl CleromancySyncBatch {
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    pub fn into_events(self) -> Vec<PersonalGraphEvent> {
        self.events
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct CleromancySyncImport {
    pub contexts: usize,
    pub readings: usize,
}

#[derive(Debug, Error)]
pub enum CleromancySyncError {
    #[error("Cleromancy sync node {node} is invalid: {reason}")]
    InvalidNode { node: Uuid, reason: String },
    #[error("synced reading {reading} has no selected context {context_digest}")]
    MissingContext {
        reading: String,
        context_digest: String,
    },
    #[error("personal graph projection has {0} operations waiting for causal history")]
    PendingHistory(usize),
    #[error("personal graph projection has an unresolved Cleromancy conflict at {0}")]
    Conflict(String),
    #[error(transparent)]
    Host(#[from] HostError),
}

#[derive(Clone)]
struct SelectedNode {
    id: Uuid,
    address: String,
    title: String,
    tags: Vec<String>,
    facet: &'static str,
    value: Value,
}

/// Project the current local graph into the generic H7 event vocabulary.
/// Nothing is authored here: the caller still owns identity, roster, durable
/// store, transport, and the moment at which this batch is published.
pub fn export_sync_batch<B: Backend>(
    host: &CleromancyHost<B>,
    selection: CleromancySyncSelection,
) -> Result<CleromancySyncBatch, CleromancySyncError> {
    let mut contexts = Vec::<(String, SelectedNode)>::new();
    let mut readings = Vec::<(Reading, SelectedNode)>::new();

    if selection.includes_contexts() {
        for (key, node) in host.graph().nodes() {
            let context = host.facet_value(key, CONTEXT_FACET);
            let reading = host.facet_value(key, READING_FACET);
            if context.is_some() && reading.is_some() {
                return Err(invalid(node.id, "carries both context and reading facets"));
            }
            if let Some(value) = context {
                let context: ContextSnapshot = serde_json::from_value(value.clone())
                    .map_err(|e| invalid(node.id, format!("context facet does not decode: {e}")))?;
                let digest = context.digest();
                validate_identity(
                    node.id,
                    node.url(),
                    &format!("cleromancy://context/{digest}"),
                )?;
                contexts.push((digest, selected_node(node, CONTEXT_FACET, value.clone())));
            } else if selection.includes_readings()
                && let Some(value) = reading
            {
                let reading: Reading = serde_json::from_value(value.clone())
                    .map_err(|e| invalid(node.id, format!("reading facet does not decode: {e}")))?;
                validate_identity(
                    node.id,
                    node.url(),
                    &format!("cleromancy://reading/{}", reading.id),
                )?;
                readings.push((reading, selected_node(node, READING_FACET, value.clone())));
            }
        }
    }

    contexts.sort_by_key(|(_, node)| node.id);
    readings.sort_by_key(|(_, node)| node.id);
    let context_ids = contexts
        .iter()
        .map(|(digest, node)| (digest.clone(), node.id))
        .collect::<BTreeMap<_, _>>();
    let mut events = Vec::new();
    for (_, node) in &contexts {
        append_node_events(&mut events, node);
    }
    for (_, node) in &readings {
        append_node_events(&mut events, node);
    }
    for (reading, node) in &readings {
        let Some(&context) = context_ids.get(&reading.receipt.context_digest) else {
            return Err(CleromancySyncError::MissingContext {
                reading: reading.id.clone(),
                context_digest: reading.receipt.context_digest.clone(),
            });
        };
        events.push(PersonalGraphEvent::AssertRelation {
            from: node.id,
            to: context,
            assertion: EdgeAssertion::Provenance {
                sub_kind: mere::kernel::graph::ProvenanceSubKind::GeneratedFrom,
            },
        });
    }

    let digest = batch_digest(selection, &events);
    Ok(CleromancySyncBatch {
        schema: SYNC_BATCH_SCHEMA,
        selection,
        events,
        contexts: contexts.len(),
        readings: readings.len(),
        digest,
    })
}

/// Merge the selected Cleromancy facets from an H7 materialization into the
/// local graph. The complete projection is validated before local mutation.
/// Deletions are deliberately not imported in A4.
pub fn import_sync_projection<B: Backend>(
    host: &mut CleromancyHost<B>,
    projection: &SyncProjection,
    selection: CleromancySyncSelection,
) -> Result<CleromancySyncImport, CleromancySyncError> {
    if matches!(selection, CleromancySyncSelection::Off) {
        return Ok(CleromancySyncImport::default());
    }
    if !projection.pending.is_empty() {
        return Err(CleromancySyncError::PendingHistory(
            projection.pending.len(),
        ));
    }
    for conflict in &projection.conflicts {
        let context = format!("/facet/{CONTEXT_FACET}");
        let reading = format!("/facet/{READING_FACET}");
        if conflict.target.ends_with(&context)
            || (selection.includes_readings() && conflict.target.ends_with(&reading))
        {
            return Err(CleromancySyncError::Conflict(conflict.target.clone()));
        }
    }

    let context_facet = FacetId::new(CONTEXT_FACET);
    let reading_facet = FacetId::new(READING_FACET);
    let mut contexts = Vec::<ContextSnapshot>::new();
    let mut readings = Vec::<Reading>::new();
    for (_, node) in projection.graph.nodes() {
        let context = projection.graph.facets().get(&node.id, &context_facet);
        let reading = projection.graph.facets().get(&node.id, &reading_facet);
        if context.is_some() && reading.is_some() {
            return Err(invalid(node.id, "carries both context and reading facets"));
        }
        if let Some(value) = context {
            let context: ContextSnapshot = serde_json::from_value(value.clone())
                .map_err(|e| invalid(node.id, format!("context facet does not decode: {e}")))?;
            validate_identity(
                node.id,
                node.url(),
                &format!("cleromancy://context/{}", context.digest()),
            )?;
            contexts.push(context);
        } else if selection.includes_readings()
            && let Some(value) = reading
        {
            let reading: Reading = serde_json::from_value(value.clone())
                .map_err(|e| invalid(node.id, format!("reading facet does not decode: {e}")))?;
            validate_identity(
                node.id,
                node.url(),
                &format!("cleromancy://reading/{}", reading.id),
            )?;
            readings.push(reading);
        }
    }
    contexts.sort_by_key(|context| context.digest());
    readings.sort_by(|left, right| left.id.cmp(&right.id));
    let contexts_by_digest = contexts
        .iter()
        .map(|context| (context.digest(), context))
        .collect::<BTreeMap<_, _>>();
    for reading in &readings {
        if !contexts_by_digest.contains_key(&reading.receipt.context_digest) {
            return Err(CleromancySyncError::MissingContext {
                reading: reading.id.clone(),
                context_digest: reading.receipt.context_digest.clone(),
            });
        }
    }

    for context in &contexts {
        host.insert_context(context)?;
    }
    for reading in &readings {
        host.insert_reading(contexts_by_digest[&reading.receipt.context_digest], reading)?;
    }
    Ok(CleromancySyncImport {
        contexts: contexts.len(),
        readings: readings.len(),
    })
}

fn selected_node(
    node: &mere::kernel::graph::Node,
    facet: &'static str,
    value: Value,
) -> SelectedNode {
    SelectedNode {
        id: node.id,
        address: node.url().to_string(),
        title: node.title.clone(),
        tags: node.tags.iter().cloned().collect(),
        facet,
        value,
    }
}

fn append_node_events(events: &mut Vec<PersonalGraphEvent>, node: &SelectedNode) {
    events.push(PersonalGraphEvent::AddNode {
        id: node.id,
        address: node.address.clone(),
        title: node.title.clone(),
    });
    let tags = node.tags.iter().cloned().collect::<BTreeSet<_>>();
    events.extend(
        tags.into_iter()
            .map(|tag| PersonalGraphEvent::AddTag { node: node.id, tag }),
    );
    events.push(PersonalGraphEvent::SetFacet {
        node: node.id,
        facet: node.facet.to_string(),
        value: node.value.clone(),
    });
}

fn validate_identity(id: Uuid, actual: &str, expected: &str) -> Result<(), CleromancySyncError> {
    if actual != expected {
        return Err(invalid(
            id,
            format!("address {actual:?} does not match {expected:?}"),
        ));
    }
    if Graph::node_namespace_id(expected) != id {
        return Err(invalid(id, "stable node id does not match its address"));
    }
    Ok(())
}

fn batch_digest(selection: CleromancySyncSelection, events: &[PersonalGraphEvent]) -> String {
    #[derive(Serialize)]
    struct DigestInput<'a> {
        schema: &'static str,
        selection: CleromancySyncSelection,
        events: &'a [PersonalGraphEvent],
    }
    let bytes = serde_json::to_vec(&DigestInput {
        schema: SYNC_BATCH_SCHEMA,
        selection,
        events,
    })
    .expect("Cleromancy sync events always serialize");
    blake3::hash(&bytes).to_hex().to_string()
}

fn invalid(node: Uuid, reason: impl Into<String>) -> CleromancySyncError {
    CleromancySyncError::InvalidNode {
        node,
        reason: reason.into(),
    }
}
