// Copyright 2026 Mark AB (markik)
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::collections::BTreeSet;

use crate::context::ContextSnapshot;
use crate::enrichment::{EnrichmentReport, tokens};
use crate::field::{
    CONTEXTUAL_WEIGHT_RULE, EXTERNAL_TERM_WEIGHT_RULE, Field, UNIFORM_DIE_RULE, UNIFORM_RULE,
};
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

/// Apply exactly the qualification rule declared by the field. Rule names are
/// executable contracts rather than labels carried only for display.
pub fn qualify(context: &ContextSnapshot, field: &Field) -> Result<QualifiedField, ReadingError> {
    match field.rules.as_str() {
        CONTEXTUAL_WEIGHT_RULE => qualify_contextual(context, field),
        UNIFORM_RULE | UNIFORM_DIE_RULE => qualify_uniform(field),
        EXTERNAL_TERM_WEIGHT_RULE => Err(ReadingError::QualificationEvidenceRequired(
            field.rules.clone(),
        )),
        _ => Err(ReadingError::UnsupportedRule(field.rules.clone())),
    }
}

/// Each matching context tag adds one base-weight share.
fn qualify_contextual(
    context: &ContextSnapshot,
    field: &Field,
) -> Result<QualifiedField, ReadingError> {
    validate_candidates(field, false)?;
    let mut weights = Vec::with_capacity(field.candidates.len());
    let mut total = 0u64;
    for candidate in &field.candidates {
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

/// Ignore context and give every candidate one share. Requiring stored base
/// weights to be one prevents a field from displaying weights the rule does
/// not actually use.
fn qualify_uniform(field: &Field) -> Result<QualifiedField, ReadingError> {
    validate_candidates(field, true)?;
    Ok(QualifiedField {
        weights: vec![1; field.candidates.len()],
        total: field.candidates.len() as u64,
    })
}

fn validate_candidates(field: &Field, require_unit_weight: bool) -> Result<(), ReadingError> {
    if field.candidates.is_empty() {
        return Err(ReadingError::EmptyField);
    }
    let mut ids = BTreeSet::new();
    for candidate in &field.candidates {
        if candidate.id.trim().is_empty() || !ids.insert(candidate.id.clone()) {
            return Err(ReadingError::InvalidCandidate(candidate.id.clone()));
        }
        if candidate.base_weight == 0 {
            return Err(ReadingError::EmptyWeight);
        }
        if require_unit_weight && candidate.base_weight != 1 {
            return Err(ReadingError::NonUniformCandidate {
                candidate: candidate.id.clone(),
                weight: candidate.base_weight,
            });
        }
    }
    Ok(())
}

/// Each distinct correlated external term declared by a candidate adds one
/// base-weight share. Repeated cards do not multiply the same term.
pub fn qualify_enriched(
    context: &ContextSnapshot,
    field: &Field,
    report: &EnrichmentReport,
) -> Result<EnrichedField, ReadingError> {
    if field.rules != EXTERNAL_TERM_WEIGHT_RULE {
        return Err(ReadingError::UnsupportedRule(field.rules.clone()));
    }
    let mut qualified = qualify_contextual(context, field)?;
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
