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

use crate::host::{CONTEXT_FACET, FIELD_FACET, READING_FACET, REFLECTION_FACET, SESSION_FACET};
use crate::{
    CleromancyHost, ContextSnapshot, Field, HostError, Reading, ReadingEngine, ReadingError,
    ReadingSession, Reflection,
};

pub const SYNC_BATCH_SCHEMA: &str = "cleromancy.sync-batch/v3";

/// The explicit local setting controlling which Cleromancy truth may enter
/// Graphshell's personal graph. Reading sync includes its contexts and exact
/// candidate fields because a receipt without either dependency cannot be
/// independently replayed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CleromancySyncSelection {
    #[default]
    Off,
    Contexts,
    /// Context, field, sealed result, and the saved reading occasion. It does
    /// not export reflective notes.
    ContextsAndReadings,
    /// The full reading history, including separately attached reflections.
    ContextsReadingsAndReflections,
}

impl CleromancySyncSelection {
    pub fn includes_contexts(self) -> bool {
        !matches!(self, Self::Off)
    }

    pub fn includes_readings(self) -> bool {
        matches!(
            self,
            Self::ContextsAndReadings | Self::ContextsReadingsAndReflections
        )
    }

    pub fn includes_sessions(self) -> bool {
        self.includes_readings()
    }

    pub fn includes_reflections(self) -> bool {
        matches!(self, Self::ContextsReadingsAndReflections)
    }

    /// Configure H7 to materialize only the named facets Cleromancy selected.
    pub fn personal_graph_selection(self) -> PersonalSyncSelection {
        let mut facets = Vec::new();
        if self.includes_contexts() {
            facets.push(CONTEXT_FACET);
        }
        if self.includes_readings() {
            facets.push(FIELD_FACET);
            facets.push(READING_FACET);
            facets.push(SESSION_FACET);
        }
        if self.includes_reflections() {
            facets.push(REFLECTION_FACET);
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
    pub fields: usize,
    pub readings: usize,
    pub sessions: usize,
    pub reflections: usize,
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
    pub fields: usize,
    pub readings: usize,
    pub sessions: usize,
    pub reflections: usize,
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
    #[error("synced reading {reading} has no selected field {field_digest}")]
    MissingField {
        reading: String,
        field_digest: String,
    },
    #[error("synced session {session} has no selected context {context_digest}")]
    MissingSessionContext {
        session: String,
        context_digest: String,
    },
    #[error("synced session {session} has no selected field {field_digest}")]
    MissingSessionField {
        session: String,
        field_digest: String,
    },
    #[error("synced session {session} has no selected reading {reading}")]
    MissingSessionReading { session: String, reading: String },
    #[error("synced reflection {reflection} has no selected session {session}")]
    MissingReflectionSession { reflection: String, session: String },
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
    let mut fields = Vec::<(String, SelectedNode)>::new();
    let mut readings = Vec::<(Reading, SelectedNode)>::new();
    let mut sessions = Vec::<(ReadingSession, SelectedNode)>::new();
    let mut reflections = Vec::<(Reflection, SelectedNode)>::new();

    if selection.includes_contexts() {
        for (key, node) in host.graph().nodes() {
            let context = host.facet_value(key, CONTEXT_FACET);
            let field = host.facet_value(key, FIELD_FACET);
            let reading = host.facet_value(key, READING_FACET);
            let session = host.facet_value(key, SESSION_FACET);
            let reflection = host.facet_value(key, REFLECTION_FACET);
            if [
                context.is_some(),
                field.is_some(),
                reading.is_some(),
                session.is_some(),
                reflection.is_some(),
            ]
            .into_iter()
            .filter(|present| *present)
            .count()
                > 1
            {
                return Err(invalid(
                    node.id,
                    "carries more than one Cleromancy domain facet",
                ));
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
                && let Some(value) = field
            {
                let field: Field = serde_json::from_value(value.clone())
                    .map_err(|e| invalid(node.id, format!("field facet does not decode: {e}")))?;
                let digest = field.digest();
                validate_identity(node.id, node.url(), &format!("cleromancy://field/{digest}"))?;
                fields.push((digest, selected_node(node, FIELD_FACET, value.clone())));
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
            } else if selection.includes_sessions()
                && let Some(value) = session
            {
                let session: ReadingSession = serde_json::from_value(value.clone())
                    .map_err(|e| invalid(node.id, format!("session facet does not decode: {e}")))?;
                session
                    .validate()
                    .map_err(|e| invalid(node.id, e.to_string()))?;
                validate_identity(
                    node.id,
                    node.url(),
                    &format!("cleromancy://session/{}", session.id),
                )?;
                sessions.push((session, selected_node(node, SESSION_FACET, value.clone())));
            } else if selection.includes_reflections()
                && let Some(value) = reflection
            {
                let reflection: Reflection =
                    serde_json::from_value(value.clone()).map_err(|e| {
                        invalid(node.id, format!("reflection facet does not decode: {e}"))
                    })?;
                reflection
                    .validate()
                    .map_err(|e| invalid(node.id, e.to_string()))?;
                validate_identity(
                    node.id,
                    node.url(),
                    &format!("cleromancy://reflection/{}", reflection.id),
                )?;
                reflections.push((
                    reflection,
                    selected_node(node, REFLECTION_FACET, value.clone()),
                ));
            }
        }
    }

    contexts.sort_by_key(|(_, node)| node.id);
    fields.sort_by_key(|(_, node)| node.id);
    readings.sort_by_key(|(_, node)| node.id);
    sessions.sort_by_key(|(_, node)| node.id);
    reflections.sort_by_key(|(_, node)| node.id);
    let context_ids = contexts
        .iter()
        .map(|(digest, node)| (digest.clone(), node.id))
        .collect::<BTreeMap<_, _>>();
    let field_ids = fields
        .iter()
        .map(|(digest, node)| (digest.clone(), node.id))
        .collect::<BTreeMap<_, _>>();
    let reading_ids = readings
        .iter()
        .map(|(reading, node)| (reading.id.clone(), node.id))
        .collect::<BTreeMap<_, _>>();
    let session_ids = sessions
        .iter()
        .map(|(session, node)| (session.id.clone(), node.id))
        .collect::<BTreeMap<_, _>>();
    let mut events = Vec::new();
    for (_, node) in &contexts {
        append_node_events(&mut events, node);
    }
    for (_, node) in &fields {
        append_node_events(&mut events, node);
    }
    for (_, node) in &readings {
        append_node_events(&mut events, node);
    }
    for (_, node) in &sessions {
        append_node_events(&mut events, node);
    }
    for (_, node) in &reflections {
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
        let Some(&field) = field_ids.get(&reading.receipt.field_digest) else {
            return Err(CleromancySyncError::MissingField {
                reading: reading.id.clone(),
                field_digest: reading.receipt.field_digest.clone(),
            });
        };
        events.push(PersonalGraphEvent::AssertRelation {
            from: node.id,
            to: field,
            assertion: EdgeAssertion::Provenance {
                sub_kind: mere::kernel::graph::ProvenanceSubKind::GeneratedFrom,
            },
        });
    }
    for (session, node) in &sessions {
        let Some(&context) = context_ids.get(&session.context_digest) else {
            return Err(CleromancySyncError::MissingSessionContext {
                session: session.id.clone(),
                context_digest: session.context_digest.clone(),
            });
        };
        events.push(PersonalGraphEvent::AssertRelation {
            from: node.id,
            to: context,
            assertion: EdgeAssertion::Provenance {
                sub_kind: mere::kernel::graph::ProvenanceSubKind::GeneratedFrom,
            },
        });
        let Some(&field) = field_ids.get(&session.field_digest) else {
            return Err(CleromancySyncError::MissingSessionField {
                session: session.id.clone(),
                field_digest: session.field_digest.clone(),
            });
        };
        events.push(PersonalGraphEvent::AssertRelation {
            from: node.id,
            to: field,
            assertion: EdgeAssertion::Provenance {
                sub_kind: mere::kernel::graph::ProvenanceSubKind::GeneratedFrom,
            },
        });
        for placement in &session.placements {
            let Some(&reading) = reading_ids.get(&placement.reading_id) else {
                return Err(CleromancySyncError::MissingSessionReading {
                    session: session.id.clone(),
                    reading: placement.reading_id.clone(),
                });
            };
            events.push(PersonalGraphEvent::AssertRelation {
                from: node.id,
                to: reading,
                assertion: EdgeAssertion::Containment {
                    sub_kind: mere::kernel::graph::ContainmentSubKind::CollectionMember,
                },
            });
        }
    }
    for (reflection, node) in &reflections {
        let Some(&session) = session_ids.get(&reflection.session_id) else {
            return Err(CleromancySyncError::MissingReflectionSession {
                reflection: reflection.id.clone(),
                session: reflection.session_id.clone(),
            });
        };
        events.push(PersonalGraphEvent::AssertRelation {
            from: node.id,
            to: session,
            assertion: EdgeAssertion::Semantic {
                sub_kind: mere::kernel::graph::SemanticSubKind::Elaborates,
                label: Some("reflects on".to_string()),
                decay_progress: None,
            },
        });
    }

    let digest = batch_digest(selection, &events);
    Ok(CleromancySyncBatch {
        schema: SYNC_BATCH_SCHEMA,
        selection,
        events,
        contexts: contexts.len(),
        fields: fields.len(),
        readings: readings.len(),
        sessions: sessions.len(),
        reflections: reflections.len(),
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
        let field = format!("/facet/{FIELD_FACET}");
        let reading = format!("/facet/{READING_FACET}");
        let session = format!("/facet/{SESSION_FACET}");
        let reflection = format!("/facet/{REFLECTION_FACET}");
        if conflict.target.ends_with(&context)
            || (selection.includes_readings()
                && (conflict.target.ends_with(&field)
                    || conflict.target.ends_with(&reading)
                    || conflict.target.ends_with(&session)))
            || (selection.includes_reflections() && conflict.target.ends_with(&reflection))
        {
            return Err(CleromancySyncError::Conflict(conflict.target.clone()));
        }
    }

    let context_facet = FacetId::new(CONTEXT_FACET);
    let field_facet = FacetId::new(FIELD_FACET);
    let reading_facet = FacetId::new(READING_FACET);
    let session_facet = FacetId::new(SESSION_FACET);
    let reflection_facet = FacetId::new(REFLECTION_FACET);
    let mut contexts = Vec::<ContextSnapshot>::new();
    let mut fields = Vec::<Field>::new();
    let mut readings = Vec::<Reading>::new();
    let mut sessions = Vec::<ReadingSession>::new();
    let mut reflections = Vec::<Reflection>::new();
    for (_, node) in projection.graph.nodes() {
        let context = projection.graph.facets().get(&node.id, &context_facet);
        let field = projection.graph.facets().get(&node.id, &field_facet);
        let reading = projection.graph.facets().get(&node.id, &reading_facet);
        let session = projection.graph.facets().get(&node.id, &session_facet);
        let reflection = projection.graph.facets().get(&node.id, &reflection_facet);
        if [
            context.is_some(),
            field.is_some(),
            reading.is_some(),
            session.is_some(),
            reflection.is_some(),
        ]
        .into_iter()
        .filter(|present| *present)
        .count()
            > 1
        {
            return Err(invalid(
                node.id,
                "carries more than one Cleromancy domain facet",
            ));
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
            && let Some(value) = field
        {
            let field: Field = serde_json::from_value(value.clone())
                .map_err(|e| invalid(node.id, format!("field facet does not decode: {e}")))?;
            validate_identity(
                node.id,
                node.url(),
                &format!("cleromancy://field/{}", field.digest()),
            )?;
            fields.push(field);
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
        } else if selection.includes_sessions()
            && let Some(value) = session
        {
            let session: ReadingSession = serde_json::from_value(value.clone())
                .map_err(|e| invalid(node.id, format!("session facet does not decode: {e}")))?;
            session
                .validate()
                .map_err(|e| invalid(node.id, e.to_string()))?;
            validate_identity(
                node.id,
                node.url(),
                &format!("cleromancy://session/{}", session.id),
            )?;
            sessions.push(session);
        } else if selection.includes_reflections()
            && let Some(value) = reflection
        {
            let reflection: Reflection = serde_json::from_value(value.clone())
                .map_err(|e| invalid(node.id, format!("reflection facet does not decode: {e}")))?;
            reflection
                .validate()
                .map_err(|e| invalid(node.id, e.to_string()))?;
            validate_identity(
                node.id,
                node.url(),
                &format!("cleromancy://reflection/{}", reflection.id),
            )?;
            reflections.push(reflection);
        }
    }
    contexts.sort_by_key(|context| context.digest());
    fields.sort_by_key(Field::digest);
    readings.sort_by(|left, right| left.id.cmp(&right.id));
    sessions.sort_by(|left, right| left.id.cmp(&right.id));
    reflections.sort_by(|left, right| left.id.cmp(&right.id));
    let contexts_by_digest = contexts
        .iter()
        .map(|context| (context.digest(), context))
        .collect::<BTreeMap<_, _>>();
    let fields_by_digest = fields
        .iter()
        .map(|field| (field.digest(), field))
        .collect::<BTreeMap<_, _>>();
    let readings_by_id = readings
        .iter()
        .map(|reading| (reading.id.clone(), reading))
        .collect::<BTreeMap<_, _>>();
    let sessions_by_id = sessions
        .iter()
        .map(|session| (session.id.clone(), session))
        .collect::<BTreeMap<_, _>>();
    for reading in &readings {
        let Some(context) = contexts_by_digest.get(&reading.receipt.context_digest) else {
            return Err(CleromancySyncError::MissingContext {
                reading: reading.id.clone(),
                context_digest: reading.receipt.context_digest.clone(),
            });
        };
        let Some(field) = fields_by_digest.get(&reading.receipt.field_digest) else {
            return Err(CleromancySyncError::MissingField {
                reading: reading.id.clone(),
                field_digest: reading.receipt.field_digest.clone(),
            });
        };
        let replayed =
            ReadingEngine::replay(context, field, &reading.receipt).map_err(HostError::from)?;
        if replayed != *reading {
            return Err(HostError::Reading(ReadingError::ReceiptMismatch(
                "sealed reading".to_string(),
            ))
            .into());
        }
    }
    for session in &sessions {
        let Some(context) = contexts_by_digest.get(&session.context_digest) else {
            return Err(CleromancySyncError::MissingSessionContext {
                session: session.id.clone(),
                context_digest: session.context_digest.clone(),
            });
        };
        let Some(field) = fields_by_digest.get(&session.field_digest) else {
            return Err(CleromancySyncError::MissingSessionField {
                session: session.id.clone(),
                field_digest: session.field_digest.clone(),
            });
        };
        for placement in &session.placements {
            let Some(reading) = readings_by_id.get(&placement.reading_id) else {
                return Err(CleromancySyncError::MissingSessionReading {
                    session: session.id.clone(),
                    reading: placement.reading_id.clone(),
                });
            };
            if reading.receipt.context_digest != session.context_digest
                || reading.receipt.field_digest != session.field_digest
            {
                return Err(invalid(
                    Graph::node_namespace_id(&format!("cleromancy://session/{}", session.id)),
                    "session placement does not share the session context and field",
                ));
            }
            let replayed =
                ReadingEngine::replay(context, field, &reading.receipt).map_err(HostError::from)?;
            if replayed != **reading {
                return Err(HostError::Reading(ReadingError::ReceiptMismatch(
                    "sealed reading".to_string(),
                ))
                .into());
            }
        }
    }
    for reflection in &reflections {
        if !sessions_by_id.contains_key(&reflection.session_id) {
            return Err(CleromancySyncError::MissingReflectionSession {
                reflection: reflection.id.clone(),
                session: reflection.session_id.clone(),
            });
        }
    }

    for context in &contexts {
        host.insert_context(context)?;
    }
    for field in &fields {
        host.insert_field(field)?;
    }
    for reading in &readings {
        host.insert_reading(
            contexts_by_digest[&reading.receipt.context_digest],
            fields_by_digest[&reading.receipt.field_digest],
            reading,
        )?;
    }
    for session in &sessions {
        let session_readings = session
            .placements
            .iter()
            .map(|placement| readings_by_id[&placement.reading_id].clone())
            .collect::<Vec<_>>();
        host.insert_session(
            contexts_by_digest[&session.context_digest],
            fields_by_digest[&session.field_digest],
            &session_readings,
            session,
        )?;
    }
    for reflection in &reflections {
        host.insert_reflection(sessions_by_id[&reflection.session_id], reflection)?;
    }
    Ok(CleromancySyncImport {
        contexts: contexts.len(),
        fields: fields.len(),
        readings: readings.len(),
        sessions: sessions.len(),
        reflections: reflections.len(),
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
