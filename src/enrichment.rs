// Copyright 2026 Mark AB (markik)
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::collections::BTreeSet;

use graphshell_client::{
    ClientState, PresentationResolution, ResolvedContent, ResolvedPresentation,
};
use graphshell_protocol::{
    CapabilityProfile, Carrier, CarrierRequestBody, CarrierResponseBody, PresentationCapability,
    ProjectionSession, ResourceRequest,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ContextSnapshot;
use crate::context::canonical_digest;

pub const CORRELATION_ALGORITHM: &str = "cleromancy.enrichment/lexical-overlap/v1";
pub const EVIDENCE_SCHEMA: &str = "cleromancy.sealed-enrichment/v1";
pub const REPORT_SCHEMA: &str = "cleromancy.enrichment-report/v2";

#[derive(Debug, Error)]
pub enum EnrichmentError {
    #[error("external Graphshell carrier: {0}")]
    Carrier(String),
    #[error("external endpoint advertised no projection")]
    NoProjection,
    #[error("external projection index {0} was not advertised")]
    UnknownProjection(usize),
    #[error("external Graphshell response was not a {0}")]
    UnexpectedResponse(&'static str),
    #[error("Graphshell refused the external snapshot: {0}")]
    Snapshot(String),
    #[error("Graphshell refused an external resource: {0}")]
    Resource(String),
    #[error("Graphshell could not resolve an external presentation: {0}")]
    Presentation(String),
    #[error("external projection disappeared from Graphshell client state")]
    MissingMount,
    #[error("sealed external evidence is invalid: {0}")]
    InvalidEvidence(String),
}

/// One mounted endpoint projection. Presentations are resolved copies of
/// disclosed resources; the source graph remains endpoint-side.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternalProjection {
    pub endpoint_label: String,
    pub projection_label: String,
    pub session: ProjectionSession,
    pub presentations: Vec<ResolvedPresentation>,
}

impl ExternalProjection {
    /// Copy exactly the portable-card strings inspected by the correlation
    /// rule. The resulting value is sufficient to replay correlation without
    /// reopening the endpoint.
    pub fn seal(&self, context: &ContextSnapshot) -> Result<SealedEnrichment, EnrichmentError> {
        let mut sources = self
            .presentations
            .iter()
            .map(EnrichmentSource::from_presentation)
            .collect::<Result<Vec<_>, _>>()?;
        sources.sort_by(|left, right| left.digest.cmp(&right.digest));
        let mut evidence = SealedEnrichment {
            schema: EVIDENCE_SCHEMA.to_string(),
            algorithm: CORRELATION_ALGORITHM.to_string(),
            context_digest: context.digest(),
            endpoint_label: self.endpoint_label.clone(),
            projection_label: self.projection_label.clone(),
            session: self.session.0.clone(),
            evidence_digest: String::new(),
            sources,
        };
        evidence.evidence_digest = evidence.expected_digest();
        Ok(evidence)
    }

    pub fn correlate(
        &self,
        context: &ContextSnapshot,
    ) -> Result<EnrichmentReport, EnrichmentError> {
        self.seal(context)?.verify(context)
    }
}

/// One portable card reduced to the exact fields read by the correlation
/// algorithm. `digest` binds those fields but does not attest who supplied
/// them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnrichmentSource {
    pub digest: String,
    pub presentation: String,
    pub title: String,
    pub badges: Vec<String>,
    pub values: Vec<EnrichmentValue>,
}

impl EnrichmentSource {
    fn from_presentation(presentation: &ResolvedPresentation) -> Result<Self, EnrichmentError> {
        let ResolvedContent::PortableCard(card) = &presentation.content else {
            return Err(EnrichmentError::InvalidEvidence(
                "the mounted representation is not a portable card".to_string(),
            ));
        };
        let mut source = Self {
            digest: String::new(),
            presentation: presentation.semantics.label.clone(),
            title: card.title.clone(),
            badges: card.badges.clone(),
            values: card
                .values
                .iter()
                .map(|value| EnrichmentValue {
                    label: value.label.clone(),
                    value: value.value.clone(),
                })
                .collect(),
        };
        source.digest = source.expected_digest();
        Ok(source)
    }

    fn expected_digest(&self) -> String {
        canonical_digest(&(&self.presentation, &self.title, &self.badges, &self.values))
    }

    fn terms(&self) -> BTreeSet<String> {
        let mut terms = tokens(&self.presentation);
        terms.extend(tokens(&self.title));
        for badge in &self.badges {
            terms.extend(tokens(badge));
        }
        for value in &self.values {
            terms.extend(tokens(&value.label));
            terms.extend(tokens(&value.value));
        }
        terms
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnrichmentValue {
    pub label: String,
    pub value: String,
}

/// A binding snapshot of the disclosed vocabulary used by qualification.
/// This is replay evidence, not a signature or an assertion of source trust.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SealedEnrichment {
    pub schema: String,
    pub algorithm: String,
    pub context_digest: String,
    pub endpoint_label: String,
    pub projection_label: String,
    pub session: String,
    pub evidence_digest: String,
    pub sources: Vec<EnrichmentSource>,
}

impl SealedEnrichment {
    pub fn verify(&self, context: &ContextSnapshot) -> Result<EnrichmentReport, EnrichmentError> {
        verify(self.schema == EVIDENCE_SCHEMA, "schema")?;
        verify(self.algorithm == CORRELATION_ALGORITHM, "algorithm")?;
        verify(self.context_digest == context.digest(), "context digest")?;
        for source in &self.sources {
            verify(source.digest == source.expected_digest(), "source digest")?;
        }
        verify(
            self.evidence_digest == self.expected_digest(),
            "evidence digest",
        )?;
        Ok(self.report(context))
    }

    fn expected_digest(&self) -> String {
        canonical_digest(&(
            &self.schema,
            &self.algorithm,
            &self.context_digest,
            &self.endpoint_label,
            &self.projection_label,
            &self.session,
            &self.sources,
        ))
    }

    fn report(&self, context: &ContextSnapshot) -> EnrichmentReport {
        let query_terms = context_terms(context);
        let mut matches = self
            .sources
            .iter()
            .filter_map(|source| {
                let terms = query_terms
                    .intersection(&source.terms())
                    .cloned()
                    .collect::<Vec<_>>();
                (!terms.is_empty()).then(|| EnrichmentMatch {
                    source_digest: source.digest.clone(),
                    presentation: source.presentation.clone(),
                    terms,
                })
            })
            .collect::<Vec<_>>();
        matches.sort_by(|left, right| {
            left.presentation
                .cmp(&right.presentation)
                .then_with(|| left.source_digest.cmp(&right.source_digest))
        });
        EnrichmentReport {
            schema: REPORT_SCHEMA.to_string(),
            algorithm: self.algorithm.clone(),
            context_digest: self.context_digest.clone(),
            endpoint_label: self.endpoint_label.clone(),
            projection_label: self.projection_label.clone(),
            session: self.session.clone(),
            query_terms: query_terms.into_iter().collect(),
            source_cards: self.sources.len(),
            matches,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnrichmentMatch {
    pub source_digest: String,
    pub presentation: String,
    pub terms: Vec<String>,
}

/// Presentation-side evidence. It is serializable for audit receipts but A1
/// never inserts it into the local Mere graph or a reading receipt.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnrichmentReport {
    pub schema: String,
    pub algorithm: String,
    pub context_digest: String,
    pub endpoint_label: String,
    pub projection_label: String,
    pub session: String,
    pub query_terms: Vec<String>,
    pub source_cards: usize,
    pub matches: Vec<EnrichmentMatch>,
}

impl EnrichmentReport {
    pub fn digest(&self) -> String {
        canonical_digest(self)
    }
}

pub(crate) fn mount_carrier(
    client: &mut ClientState,
    carrier: &mut impl Carrier,
    projection_index: usize,
) -> Result<ExternalProjection, EnrichmentError> {
    let descriptor = match carrier
        .request(CarrierRequestBody::Discover)
        .map_err(EnrichmentError::Carrier)?
    {
        CarrierResponseBody::Descriptor(descriptor) => descriptor,
        _ => return Err(EnrichmentError::UnexpectedResponse("descriptor")),
    };
    if descriptor.projections.is_empty() {
        return Err(EnrichmentError::NoProjection);
    }
    let offer = descriptor
        .projections
        .get(projection_index)
        .cloned()
        .ok_or(EnrichmentError::UnknownProjection(projection_index))?;
    let snapshot = match carrier
        .request(CarrierRequestBody::Snapshot(offer.request))
        .map_err(EnrichmentError::Carrier)?
    {
        CarrierResponseBody::Snapshot(snapshot) => *snapshot,
        _ => return Err(EnrichmentError::UnexpectedResponse("snapshot")),
    };
    let session = snapshot.session.clone();
    let resources = snapshot
        .presentation
        .offers
        .values()
        .flatten()
        .map(|offer| offer.resource)
        .collect::<BTreeSet<_>>();
    client
        .apply_snapshot(snapshot)
        .map_err(|error| EnrichmentError::Snapshot(format!("{error:?}")))?;
    for resource in resources {
        let response = match carrier
            .request(CarrierRequestBody::Resource(ResourceRequest {
                session: session.clone(),
                resource,
            }))
            .map_err(EnrichmentError::Carrier)?
        {
            CarrierResponseBody::Resource(response) => response,
            _ => return Err(EnrichmentError::UnexpectedResponse("resource")),
        };
        client
            .apply_resource(response)
            .map_err(|error| EnrichmentError::Resource(format!("{error:?}")))?;
    }

    let mounted = client
        .mounted(&session)
        .ok_or(EnrichmentError::MissingMount)?;
    let instances = mounted
        .scene
        .active_items_in_order()
        .into_iter()
        .map(|(instance, _)| instance)
        .collect::<Vec<_>>();
    let profile = CapabilityProfile::new([PresentationCapability::PortableCard]);
    let presentations = instances
        .into_iter()
        .map(
            |instance| match client.resolve(&session, instance, &profile) {
                Ok(PresentationResolution::Ready(presentation)) => Ok(presentation),
                Ok(PresentationResolution::NeedsResource(_)) => Err(EnrichmentError::Presentation(
                    "advertised resource was not fetched".to_string(),
                )),
                Err(error) => Err(EnrichmentError::Presentation(format!("{error:?}"))),
            },
        )
        .collect::<Result<Vec<_>, _>>()?;

    Ok(ExternalProjection {
        endpoint_label: descriptor.label,
        projection_label: offer.label,
        session,
        presentations,
    })
}

fn context_terms(context: &ContextSnapshot) -> BTreeSet<String> {
    let mut terms = tokens(&context.label);
    for tag in &context.tags {
        terms.extend(tokens(tag));
    }
    for (name, value) in &context.facts {
        terms.extend(tokens(name));
        terms.extend(tokens(value));
    }
    terms
}

pub(crate) fn tokens(value: &str) -> BTreeSet<String> {
    value
        .split(|character: char| !character.is_alphanumeric())
        .map(str::to_lowercase)
        .filter(|term| term.chars().count() >= 3)
        .collect()
}

fn verify(condition: bool, field: &str) -> Result<(), EnrichmentError> {
    condition.then_some(()).ok_or_else(|| {
        EnrichmentError::InvalidEvidence(format!("{field} does not match its contents"))
    })
}
