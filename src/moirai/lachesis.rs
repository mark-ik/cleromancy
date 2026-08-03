// Copyright 2026 Mark AB (markik)
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::collections::BTreeSet;

use crate::context::ContextSnapshot;
use crate::enrichment::{EnrichmentReport, tokens};
use crate::field::Field;
use crate::reading::ReadingError;

/// The field after declared context qualification.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QualifiedField {
    pub weights: Vec<u64>,
    pub total: u64,
}

/// The externally qualified field plus the exact transparent additions used
/// to obtain it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnrichedField {
    pub qualified: QualifiedField,
    pub candidate_terms: Vec<Vec<String>>,
    pub weight_additions: Vec<u64>,
}

/// A0's explicit rule: each matching context tag adds one base-weight share.
/// Systems can later supply their own versioned qualifier behind this seam.
pub fn qualify(context: &ContextSnapshot, field: &Field) -> Result<QualifiedField, ReadingError> {
    if field.candidates.is_empty() {
        return Err(ReadingError::EmptyField);
    }
    let mut ids = BTreeSet::new();
    let mut weights = Vec::with_capacity(field.candidates.len());
    let mut total = 0u64;
    for candidate in &field.candidates {
        if candidate.id.trim().is_empty() || !ids.insert(candidate.id.clone()) {
            return Err(ReadingError::InvalidCandidate(candidate.id.clone()));
        }
        if candidate.base_weight == 0 {
            return Err(ReadingError::EmptyWeight);
        }
        let matches = candidate.tags.intersection(&context.tags).count() as u64;
        let weight = candidate
            .base_weight
            .checked_mul(matches + 1)
            .ok_or(ReadingError::WeightOverflow)?;
        total = total
            .checked_add(weight)
            .ok_or(ReadingError::WeightOverflow)?;
        weights.push(weight);
    }
    Ok(QualifiedField { weights, total })
}

/// Each distinct correlated external term declared by a candidate adds one
/// base-weight share. Repeated cards do not multiply the same term.
pub fn qualify_enriched(
    context: &ContextSnapshot,
    field: &Field,
    report: &EnrichmentReport,
) -> Result<EnrichedField, ReadingError> {
    let mut qualified = qualify(context, field)?;
    let evidence_terms = report
        .matches
        .iter()
        .flat_map(|matched| matched.terms.iter().cloned())
        .collect::<BTreeSet<_>>();
    let mut candidate_terms = Vec::with_capacity(field.candidates.len());
    let mut weight_additions = Vec::with_capacity(field.candidates.len());
    for (index, candidate) in field.candidates.iter().enumerate() {
        let declared_terms = candidate
            .tags
            .iter()
            .flat_map(|tag| tokens(tag).into_iter())
            .collect::<BTreeSet<_>>();
        let matches = declared_terms
            .intersection(&evidence_terms)
            .cloned()
            .collect::<Vec<_>>();
        let addition = candidate
            .base_weight
            .checked_mul(matches.len() as u64)
            .ok_or(ReadingError::WeightOverflow)?;
        qualified.weights[index] = qualified.weights[index]
            .checked_add(addition)
            .ok_or(ReadingError::WeightOverflow)?;
        qualified.total = qualified
            .total
            .checked_add(addition)
            .ok_or(ReadingError::WeightOverflow)?;
        candidate_terms.push(matches);
        weight_additions.push(addition);
    }
    Ok(EnrichedField {
        qualified,
        candidate_terms,
        weight_additions,
    })
}

pub fn calculated_index(field: &QualifiedField) -> Result<usize, ReadingError> {
    field
        .weights
        .iter()
        .enumerate()
        .max_by_key(|(index, weight)| (**weight, std::cmp::Reverse(*index)))
        .map(|(index, _)| index)
        .ok_or(ReadingError::EmptyField)
}

pub fn index_for_sample(field: &QualifiedField, sample: u64) -> Result<usize, ReadingError> {
    if sample >= field.total {
        return Err(ReadingError::InvalidSample {
            sample,
            upper: field.total,
        });
    }
    let mut cursor = sample;
    for (index, weight) in field.weights.iter().copied().enumerate() {
        if cursor < weight {
            return Ok(index);
        }
        cursor -= weight;
    }
    Err(ReadingError::InvalidSample {
        sample,
        upper: field.total,
    })
}
