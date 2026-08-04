// Copyright 2026 Mark AB (markik)
// SPDX-License-Identifier: MIT OR Apache-2.0

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{Candidate, Field};

pub const FIELD_COMPOSER_SCHEMA: &str = "cleromancy.field-composer/v1";

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ComposerError {
    #[error("field system is empty")]
    EmptySystem,
    #[error("field rules are empty")]
    EmptyRules,
    #[error("field needs at least one candidate")]
    EmptyField,
    #[error("candidate id is empty or duplicated: {0}")]
    InvalidCandidate(String),
    #[error("candidate {0} must have nonzero base weight")]
    EmptyWeight(String),
}

/// A product-owned draft for a generic candidate field.
///
/// This is deliberately a structural authoring layer. It does not invent
/// interpretations, choose a qualification rule, or claim that the declared
/// rule is executable. `finish` emits the ordinary exact `Field` that the
/// reading engine and composition intent already retain and replay.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldComposer {
    pub schema: String,
    pub system: String,
    pub rules: String,
    pub candidates: Vec<Candidate>,
}

impl FieldComposer {
    pub fn new(system: impl Into<String>, rules: impl Into<String>) -> Self {
        Self {
            schema: FIELD_COMPOSER_SCHEMA.to_string(),
            system: system.into(),
            rules: rules.into(),
            candidates: Vec::new(),
        }
    }

    pub fn add_candidate(&mut self, candidate: Candidate) -> Result<(), ComposerError> {
        validate_candidate(&candidate, &self.candidates)?;
        self.candidates.push(candidate);
        Ok(())
    }

    pub fn with_candidate(mut self, candidate: Candidate) -> Result<Self, ComposerError> {
        self.add_candidate(candidate)?;
        Ok(self)
    }

    pub fn finish(self) -> Result<Field, ComposerError> {
        if self.system.trim().is_empty() {
            return Err(ComposerError::EmptySystem);
        }
        if self.rules.trim().is_empty() {
            return Err(ComposerError::EmptyRules);
        }
        if self.candidates.is_empty() {
            return Err(ComposerError::EmptyField);
        }
        let mut seen = Vec::with_capacity(self.candidates.len());
        for candidate in &self.candidates {
            validate_candidate(candidate, &seen)?;
            seen.push(candidate.clone());
        }
        Ok(Field::new(self.system, self.rules, self.candidates))
    }
}

fn validate_candidate(candidate: &Candidate, existing: &[Candidate]) -> Result<(), ComposerError> {
    if candidate.id.trim().is_empty() || existing.iter().any(|prior| prior.id == candidate.id) {
        return Err(ComposerError::InvalidCandidate(candidate.id.clone()));
    }
    if candidate.base_weight == 0 {
        return Err(ComposerError::EmptyWeight(candidate.id.clone()));
    }
    Ok(())
}
