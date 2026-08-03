// Copyright 2026 Mark AB (markik)
// SPDX-License-Identifier: MIT OR Apache-2.0

use graphshell::view::{ProjectionLayoutView, ProjectionReceiptView, render_projection_receipt};
use graphshell_client::{
    ClientState, PresentationResolution, ResolvedPresentation, SnapshotApplyError,
};
use graphshell_endpoint::PresentationSource;
use graphshell_protocol::{
    CapabilityProfile, CarrierNotice, IntentInvocation, IntentResult, PresentationCapability,
};
use muniment::Backend;
use servitor::Subject;
use thiserror::Error;

use crate::enrichment::{EnrichmentError, EnrichmentReport, ExternalProjection, mount_carrier};
use crate::intents::{
    IntentLimits, READ_INTENT, READ_SCHEMA, ROLL_INTENT, ROLL_SCHEMA, ReadingIntentPayload,
    RollIntentPayload, SELECT_INTENT, SELECT_SCHEMA, die_field, scope_for,
};
use crate::moirai::clotho::{EntropySource, OsEntropy};
use crate::{CleromancyHost, HostError, ReadingEngine, ReadingError, ServitorAccess};

#[derive(Debug, Error)]
pub enum AppError {
    #[error(transparent)]
    Host(#[from] HostError),
    #[error("Graphshell refused the projection snapshot: {0:?}")]
    Snapshot(SnapshotApplyError),
    #[error("Graphshell client: {0}")]
    Client(String),
    #[error("Cleromancy intent: {0}")]
    Intent(String),
}

/// A small product composition: Cleromancy owns readings, Graphshell owns the
/// portable presentation client, and Servitor stays reachable at an explicit
/// authority seam.
pub struct CleromancyApp<B> {
    pub host: CleromancyHost<B>,
    client: ClientState,
    servitors: ServitorAccess,
    intent_subject: Option<Subject>,
    intent_limits: IntentLimits,
    pending_notice: bool,
}

impl<B: Backend> CleromancyApp<B> {
    pub fn new(host: CleromancyHost<B>) -> Self {
        Self {
            host,
            client: ClientState::default(),
            servitors: ServitorAccess::new(),
            intent_subject: None,
            intent_limits: IntentLimits::default(),
            pending_notice: false,
        }
    }

    pub fn client(&self) -> &ClientState {
        &self.client
    }

    pub fn servitors(&self) -> &ServitorAccess {
        &self.servitors
    }

    pub fn servitors_mut(&mut self) -> &mut ServitorAccess {
        &mut self.servitors
    }

    /// Bind the peer identity proved by the containing transport. Intent
    /// payloads never get to assert their own subject.
    pub fn bind_intent_subject(&mut self, subject: Subject) {
        self.intent_subject = Some(subject);
    }

    pub fn clear_intent_subject(&mut self) {
        self.intent_subject = None;
    }

    pub fn intent_limits(&self) -> IntentLimits {
        self.intent_limits
    }

    pub fn set_intent_limits(&mut self, limits: IntentLimits) {
        self.intent_limits = limits;
    }

    pub(crate) fn intents_are_bound(&self) -> bool {
        self.intent_subject.is_some()
    }

    /// Lower an advertised Graphshell command through the bound Servitor
    /// subject. Production dispatch supplies OS entropy; tests may inject a
    /// deterministic source without creating a second command path.
    pub fn invoke_with_entropy(
        &mut self,
        intent: IntentInvocation,
        entropy: &mut impl EntropySource,
    ) -> Result<IntentResult, AppError> {
        if intent.session != self.host.session() {
            return Err(AppError::Intent(
                "intent names another projection session".to_string(),
            ));
        }
        let Some((epoch, revision)) = self.host.active_revision() else {
            return Err(AppError::Intent(
                "intent arrived before a projection snapshot".to_string(),
            ));
        };
        if intent.observed_epoch != epoch || intent.observed_revision != revision {
            return Ok(IntentResult::Stale {
                current_epoch: epoch,
                current_revision: revision,
            });
        }
        if !self
            .host
            .intent_was_advertised(intent.target, &intent.intent)
        {
            return Ok(rejected(
                "target or intent was not advertised by this snapshot",
            ));
        }
        if intent.payload.len() > self.intent_limits.max_payload_bytes {
            return Ok(rejected("payload exceeds the configured byte limit"));
        }
        let Some(subject) = self.intent_subject else {
            return Ok(rejected(
                "the containing transport did not bind an authenticated subject",
            ));
        };
        let context = self
            .host
            .context_for_instance(intent.target)
            .ok_or_else(|| AppError::Intent("advertised context disappeared".to_string()))?;
        let scope = scope_for(&intent.intent)
            .ok_or_else(|| AppError::Intent("advertised intent has no scope".to_string()))?;

        let reading = match intent.intent.as_str() {
            READ_INTENT | SELECT_INTENT => {
                let payload = match serde_json::from_slice::<ReadingIntentPayload>(&intent.payload)
                {
                    Ok(payload) => payload,
                    Err(error) => return Ok(rejected(format!("invalid reading payload: {error}"))),
                };
                let expected_schema = if intent.intent == READ_INTENT {
                    READ_SCHEMA
                } else {
                    SELECT_SCHEMA
                };
                if payload.schema != expected_schema {
                    return Ok(rejected("reading payload schema does not match the intent"));
                }
                if payload.field.candidates.is_empty()
                    || payload.field.candidates.len() > self.intent_limits.max_candidates
                {
                    return Ok(rejected(
                        "candidate count is outside the configured intent limits",
                    ));
                }
                if self
                    .servitors
                    .petition_write(subject, scope, format!("{} request", intent.intent))
                    .is_err()
                {
                    return Ok(rejected("Servitor refused the bound subject"));
                }
                let result = match (intent.intent.as_str(), payload.enrichment.as_ref()) {
                    (READ_INTENT, Some(evidence)) => {
                        ReadingEngine::calculate_enriched(&context, &payload.field, evidence)
                    }
                    (READ_INTENT, None) => ReadingEngine::calculate(&context, &payload.field),
                    (SELECT_INTENT, Some(evidence)) => ReadingEngine::cast_enriched_with(
                        &context,
                        &payload.field,
                        evidence,
                        entropy,
                    ),
                    (SELECT_INTENT, None) => {
                        ReadingEngine::cast_with(&context, &payload.field, entropy)
                    }
                    _ => unreachable!("the match is limited to read and select"),
                };
                match result {
                    Ok(reading) => reading,
                    Err(ReadingError::Entropy(error)) => {
                        return Err(AppError::Intent(format!("entropy failed: {error}")));
                    }
                    Err(error) => {
                        return Ok(rejected(format!("reading request is invalid: {error}")));
                    }
                }
            }
            ROLL_INTENT => {
                let payload = match serde_json::from_slice::<RollIntentPayload>(&intent.payload) {
                    Ok(payload) => payload,
                    Err(error) => return Ok(rejected(format!("invalid roll payload: {error}"))),
                };
                if payload.schema != ROLL_SCHEMA {
                    return Ok(rejected("roll payload schema does not match the intent"));
                }
                if payload.sides < 2 || payload.sides > self.intent_limits.max_die_sides {
                    return Ok(rejected(
                        "die sides are outside the configured intent limits",
                    ));
                }
                if self
                    .servitors
                    .petition_write(subject, scope, "cleromancy.roll request")
                    .is_err()
                {
                    return Ok(rejected("Servitor refused the bound subject"));
                }
                let field = die_field(payload.sides, payload.label.as_deref());
                match ReadingEngine::cast_with(&context, &field, entropy) {
                    Ok(reading) => reading,
                    Err(ReadingError::Entropy(error)) => {
                        return Err(AppError::Intent(format!("entropy failed: {error}")));
                    }
                    Err(error) => {
                        return Ok(rejected(format!("roll request is invalid: {error}")));
                    }
                }
            }
            _ => return Ok(rejected("intent is not implemented")),
        };
        self.host.insert_reading(&context, &reading)?;
        self.pending_notice = true;
        Ok(IntentResult::Accepted)
    }

    pub(crate) fn take_projection_notice(&mut self) -> Option<CarrierNotice> {
        if !std::mem::take(&mut self.pending_notice) {
            return None;
        }
        let (epoch, revision) = self.host.active_revision()?;
        Some(CarrierNotice {
            session: self.host.session(),
            epoch,
            revision,
        })
    }

    pub fn invoke(&mut self, intent: IntentInvocation) -> Result<IntentResult, AppError> {
        self.invoke_with_entropy(intent, &mut OsEntropy)
    }

    /// Mount the local endpoint through the same snapshot/resource split a
    /// remote Graphshell endpoint uses.
    pub fn mount_local(&mut self) -> Result<Vec<ResolvedPresentation>, AppError> {
        let session = self.host.session();
        let snapshot = self
            .host
            .build_snapshot_with_actions(self.intents_are_bound())?;
        let resources = snapshot
            .presentation
            .offers
            .values()
            .flatten()
            .map(|offer| offer.resource)
            .collect::<std::collections::BTreeSet<_>>();
        self.client
            .apply_snapshot(snapshot)
            .map_err(AppError::Snapshot)?;
        for resource in resources {
            let response = self.host.resource(graphshell_protocol::ResourceRequest {
                session: session.clone(),
                resource,
            })?;
            self.client
                .apply_resource(response)
                .map_err(|error| AppError::Client(format!("resource: {error:?}")))?;
        }

        let mounted = self
            .client
            .mounted(&session)
            .ok_or_else(|| AppError::Client("mounted scene disappeared".to_string()))?;
        let instances = mounted
            .scene
            .active_items_in_order()
            .into_iter()
            .map(|(instance, _)| instance)
            .collect::<Vec<_>>();
        let profile = CapabilityProfile::new([PresentationCapability::PortableCard]);
        instances
            .into_iter()
            .map(
                |instance| match self.client.resolve(&session, instance, &profile) {
                    Ok(PresentationResolution::Ready(presentation)) => Ok(presentation),
                    Ok(PresentationResolution::NeedsResource(_)) => Err(AppError::Client(
                        "advertised presentation resource was not fetched".to_string(),
                    )),
                    Err(error) => Err(AppError::Client(format!("presentation: {error:?}"))),
                },
            )
            .collect()
    }

    pub fn receipt_html(&mut self) -> Result<String, AppError> {
        let presentations = self.mount_local()?;
        let session = self.host.session();
        let mounted = self
            .client
            .mounted(&session)
            .ok_or_else(|| AppError::Client("mounted scene disappeared".to_string()))?;
        Ok(render_projection_receipt(&ProjectionReceiptView {
            eyebrow: "Cleromancy · local reading graph".to_string(),
            title: "Qualified readings, with their workings".to_string(),
            lede: "The context and calculation remain visible. Externally qualified readings carry the disclosed cards and exact weight additions; cast readings disclose a bounded random sample without claiming that entropy is meaning.".to_string(),
            session: session.0,
            status: format!("{:?}", mounted.status).to_lowercase(),
            presentations,
            layout: Some(ProjectionLayoutView::from_scene(&mounted.scene)),
            intents: Vec::new(),
        }))
    }

    /// Mount a source-owned projection in the same Graphshell client as the
    /// local reading graph. No external node or facet enters `self.host`.
    pub fn mount_external(
        &mut self,
        carrier: &mut impl graphshell_protocol::Carrier,
        projection_index: usize,
    ) -> Result<ExternalProjection, EnrichmentError> {
        mount_carrier(&mut self.client, carrier, projection_index)
    }

    pub fn enrichment_receipt_html(
        &self,
        projection: &ExternalProjection,
        report: &EnrichmentReport,
    ) -> Result<String, EnrichmentError> {
        let mounted = self
            .client
            .mounted(&projection.session)
            .ok_or(EnrichmentError::MissingMount)?;
        let matches = report
            .matches
            .iter()
            .map(|matched| {
                let digest = matched.source_digest.chars().take(12).collect::<String>();
                format!(
                    "{}#{} [{}]",
                    matched.presentation,
                    digest,
                    matched.terms.join(", ")
                )
            })
            .collect::<Vec<_>>();
        let correlation = if matches.is_empty() {
            "No lexical overlap was found.".to_string()
        } else {
            format!("Disclosed overlaps: {}.", matches.join("; "))
        };
        Ok(render_projection_receipt(&ProjectionReceiptView {
            eyebrow: "Cleromancy A1 · external Graphshell projection".to_string(),
            title: format!("{} remains source-owned", projection.endpoint_label),
            lede: format!(
                "{} Cleromancy computed {} and did not import the source graph or change reading weights.",
                correlation, report.algorithm
            ),
            session: projection.session.0.clone(),
            status: format!("{:?}", mounted.status).to_lowercase(),
            presentations: projection.presentations.clone(),
            layout: Some(ProjectionLayoutView::from_scene(&mounted.scene)),
            intents: Vec::new(),
        }))
    }
}

fn rejected(reason: impl Into<String>) -> IntentResult {
    IntentResult::Rejected {
        reason: reason.into(),
    }
}
