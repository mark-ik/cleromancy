// Copyright 2026 Mark AB (markik)
// SPDX-License-Identifier: MIT OR Apache-2.0

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::context::ContextSnapshot;
use crate::enrichment::SealedEnrichment;
use crate::field::Field;
use crate::moirai::{atropos, clotho, lachesis};

pub const EXTERNAL_QUALIFICATION_ALGORITHM: &str =
    "cleromancy.qualification/external-term-share/v1";

/// Whether a reading follows the declared qualifier to its maximum or casts
/// within the same qualified field.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SelectionMode {
    Calculated,
    Cast,
}

/// The sealed source snapshot and every derived addition used to qualify the
/// candidate field. Replay recomputes these fields from `evidence`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnrichmentQualification {
    pub schema: String,
    pub algorithm: String,
    pub evidence: SealedEnrichment,
    pub report_digest: String,
    pub candidate_terms: Vec<Vec<String>>,
    pub weight_additions: Vec<u64>,
}

/// The complete, replayable calculation disclosed beside a reading.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Receipt {
    pub schema: String,
    pub mode: SelectionMode,
    pub algorithm: String,
    pub context_digest: String,
    pub field_digest: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enrichment: Option<EnrichmentQualification>,
    pub qualified_weights: Vec<u64>,
    pub total_weight: u64,
    pub sample: Option<u64>,
    pub event_nonce: Option<String>,
    pub selected_index: usize,
    pub selected_candidate: String,
}

/// A sealed result plus the evidence needed to audit it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reading {
    pub schema: String,
    pub id: String,
    pub system: String,
    pub candidate_id: String,
    pub title: String,
    pub interpretation: String,
    pub receipt: Receipt,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ReadingError {
    #[error("the system has no candidates")]
    EmptyField,
    #[error("qualified weight must be nonzero")]
    EmptyWeight,
    #[error("candidate id is empty or duplicated: {0}")]
    InvalidCandidate(String),
    #[error("qualified weight overflowed u64")]
    WeightOverflow,
    #[error("operating-system entropy failed: {0}")]
    Entropy(String),
    #[error("sample {sample} is outside 0..{upper}")]
    InvalidSample { sample: u64, upper: u64 },
    #[error("receipt does not match its declared inputs: {0}")]
    ReceiptMismatch(String),
    #[error("sealed enrichment does not match its declared inputs: {0}")]
    InvalidEnrichment(String),
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ReadingEngine;

impl ReadingEngine {
    pub fn calculate(context: &ContextSnapshot, field: &Field) -> Result<Reading, ReadingError> {
        let qualified = lachesis::qualify(context, field)?;
        let index = lachesis::calculated_index(&qualified)?;
        Ok(atropos::seal(
            field,
            &field.candidates[index],
            make_receipt(
                context,
                field,
                &qualified,
                index,
                SelectionProof::calculated(),
                None,
            ),
        ))
    }

    pub fn calculate_enriched(
        context: &ContextSnapshot,
        field: &Field,
        evidence: &SealedEnrichment,
    ) -> Result<Reading, ReadingError> {
        let (qualified, enrichment) = prepare_enrichment(context, field, evidence)?;
        let index = lachesis::calculated_index(&qualified)?;
        Ok(atropos::seal(
            field,
            &field.candidates[index],
            make_receipt(
                context,
                field,
                &qualified,
                index,
                SelectionProof::calculated(),
                Some(enrichment),
            ),
        ))
    }

    pub fn cast(context: &ContextSnapshot, field: &Field) -> Result<Reading, ReadingError> {
        Self::cast_with(context, field, &mut clotho::OsEntropy)
    }

    pub fn cast_with(
        context: &ContextSnapshot,
        field: &Field,
        entropy: &mut impl clotho::EntropySource,
    ) -> Result<Reading, ReadingError> {
        let qualified = lachesis::qualify(context, field)?;
        let draw = clotho::draw_below(entropy, qualified.total)?;
        let index = lachesis::index_for_sample(&qualified, draw.sample)?;
        Ok(atropos::seal(
            field,
            &field.candidates[index],
            make_receipt(
                context,
                field,
                &qualified,
                index,
                SelectionProof::cast(draw.sample, draw.event_nonce),
                None,
            ),
        ))
    }

    pub fn cast_enriched(
        context: &ContextSnapshot,
        field: &Field,
        evidence: &SealedEnrichment,
    ) -> Result<Reading, ReadingError> {
        Self::cast_enriched_with(context, field, evidence, &mut clotho::OsEntropy)
    }

    pub fn cast_enriched_with(
        context: &ContextSnapshot,
        field: &Field,
        evidence: &SealedEnrichment,
        entropy: &mut impl clotho::EntropySource,
    ) -> Result<Reading, ReadingError> {
        let (qualified, enrichment) = prepare_enrichment(context, field, evidence)?;
        let draw = clotho::draw_below(entropy, qualified.total)?;
        let index = lachesis::index_for_sample(&qualified, draw.sample)?;
        Ok(atropos::seal(
            field,
            &field.candidates[index],
            make_receipt(
                context,
                field,
                &qualified,
                index,
                SelectionProof::cast(draw.sample, draw.event_nonce),
                Some(enrichment),
            ),
        ))
    }

    /// Recompute every derivable field. Cast replay uses the already-bounded
    /// sample disclosed by the receipt and never pretends to recover entropy.
    pub fn replay(
        context: &ContextSnapshot,
        field: &Field,
        receipt: &Receipt,
    ) -> Result<Reading, ReadingError> {
        check(receipt.context_digest == context.digest(), "context digest")?;
        check(receipt.field_digest == field.digest(), "field digest")?;
        let qualified = match &receipt.enrichment {
            Some(enrichment) => replay_enrichment(context, field, enrichment)?,
            None => lachesis::qualify(context, field)?,
        };
        check(
            receipt.qualified_weights == qualified.weights,
            "qualified weights",
        )?;
        check(receipt.total_weight == qualified.total, "total weight")?;
        let index = match receipt.mode {
            SelectionMode::Calculated => {
                check(receipt.sample.is_none(), "calculated sample")?;
                check(receipt.event_nonce.is_none(), "calculated nonce")?;
                lachesis::calculated_index(&qualified)?
            }
            SelectionMode::Cast => {
                let sample = receipt
                    .sample
                    .ok_or_else(|| ReadingError::ReceiptMismatch("cast sample".to_string()))?;
                check(receipt.event_nonce.is_some(), "cast nonce")?;
                lachesis::index_for_sample(&qualified, sample)?
            }
        };
        check(receipt.selected_index == index, "selected index")?;
        check(
            receipt.selected_candidate == field.candidates[index].id,
            "selected candidate",
        )?;
        let rebuilt = make_receipt(
            context,
            field,
            &qualified,
            index,
            SelectionProof {
                mode: receipt.mode,
                sample: receipt.sample,
                event_nonce: receipt.event_nonce.clone(),
            },
            receipt.enrichment.clone(),
        );
        check(rebuilt == *receipt, "algorithm or schema")?;
        Ok(atropos::seal(field, &field.candidates[index], rebuilt))
    }
}

struct SelectionProof {
    mode: SelectionMode,
    sample: Option<u64>,
    event_nonce: Option<String>,
}

impl SelectionProof {
    fn calculated() -> Self {
        Self {
            mode: SelectionMode::Calculated,
            sample: None,
            event_nonce: None,
        }
    }

    fn cast(sample: u64, event_nonce: String) -> Self {
        Self {
            mode: SelectionMode::Cast,
            sample: Some(sample),
            event_nonce: Some(event_nonce),
        }
    }
}

fn make_receipt(
    context: &ContextSnapshot,
    field: &Field,
    qualified: &lachesis::QualifiedField,
    index: usize,
    selection: SelectionProof,
    enrichment: Option<EnrichmentQualification>,
) -> Receipt {
    let enriched = enrichment.is_some();
    let mode = selection.mode;
    Receipt {
        schema: if enriched {
            "cleromancy.receipt/v2"
        } else {
            "cleromancy.receipt/v1"
        }
        .to_string(),
        mode,
        algorithm: match (mode, enriched) {
            (SelectionMode::Calculated, false) => "contextual-weight/max/v1",
            (SelectionMode::Cast, false) => "os-csprng/rejection-u64+contextual-weight/v1",
            (SelectionMode::Calculated, true) => "contextual-weight+external-term-share/max/v1",
            (SelectionMode::Cast, true) => {
                "os-csprng/rejection-u64+contextual-weight+external-term-share/v1"
            }
        }
        .to_string(),
        context_digest: context.digest(),
        field_digest: field.digest(),
        enrichment,
        qualified_weights: qualified.weights.clone(),
        total_weight: qualified.total,
        sample: selection.sample,
        event_nonce: selection.event_nonce,
        selected_index: index,
        selected_candidate: field.candidates[index].id.clone(),
    }
}

fn prepare_enrichment(
    context: &ContextSnapshot,
    field: &Field,
    evidence: &SealedEnrichment,
) -> Result<(lachesis::QualifiedField, EnrichmentQualification), ReadingError> {
    let report = evidence
        .verify(context)
        .map_err(|error| ReadingError::InvalidEnrichment(error.to_string()))?;
    let enriched = lachesis::qualify_enriched(context, field, &report)?;
    let qualification = EnrichmentQualification {
        schema: "cleromancy.enrichment-qualification/v1".to_string(),
        algorithm: EXTERNAL_QUALIFICATION_ALGORITHM.to_string(),
        evidence: evidence.clone(),
        report_digest: report.digest(),
        candidate_terms: enriched.candidate_terms,
        weight_additions: enriched.weight_additions,
    };
    Ok((enriched.qualified, qualification))
}

fn replay_enrichment(
    context: &ContextSnapshot,
    field: &Field,
    qualification: &EnrichmentQualification,
) -> Result<lachesis::QualifiedField, ReadingError> {
    check(
        qualification.schema == "cleromancy.enrichment-qualification/v1",
        "enrichment qualification schema",
    )?;
    check(
        qualification.algorithm == EXTERNAL_QUALIFICATION_ALGORITHM,
        "enrichment qualification algorithm",
    )?;
    let (qualified, rebuilt) = prepare_enrichment(context, field, &qualification.evidence)?;
    check(
        rebuilt.report_digest == qualification.report_digest,
        "enrichment report digest",
    )?;
    check(
        rebuilt.candidate_terms == qualification.candidate_terms,
        "enrichment candidate terms",
    )?;
    check(
        rebuilt.weight_additions == qualification.weight_additions,
        "enrichment weight additions",
    )?;
    Ok(qualified)
}

fn check(condition: bool, field: &str) -> Result<(), ReadingError> {
    condition
        .then_some(())
        .ok_or_else(|| ReadingError::ReceiptMismatch(field.to_string()))
}
