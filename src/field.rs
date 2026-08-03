// Copyright 2026 Mark AB (markik)
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::context::canonical_digest;

pub const CONTEXTUAL_WEIGHT_RULE: &str = "contextual-weight/v1";
pub const EXTERNAL_TERM_WEIGHT_RULE: &str = "contextual-weight+external-term-share/v1";
pub const UNIFORM_RULE: &str = "uniform/v1";
pub const UNIFORM_DIE_RULE: &str = "uniform-die/v1";

/// One possible suggestion or interpretive focus.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Candidate {
    pub id: String,
    pub title: String,
    pub interpretation: String,
    pub tags: BTreeSet<String>,
    pub base_weight: u64,
}

impl Candidate {
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        interpretation: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            interpretation: interpretation.into(),
            tags: BTreeSet::new(),
            base_weight: 1,
        }
    }

    pub fn with_tags(mut self, tags: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.tags.extend(tags.into_iter().map(Into::into));
        self
    }

    pub fn with_base_weight(mut self, weight: u64) -> Self {
        self.base_weight = weight;
        self
    }
}

/// A system's candidate space and the declared qualification rule version.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Field {
    pub system: String,
    pub rules: String,
    pub candidates: Vec<Candidate>,
}

impl Field {
    pub fn new(
        system: impl Into<String>,
        rules: impl Into<String>,
        candidates: impl IntoIterator<Item = Candidate>,
    ) -> Self {
        Self {
            system: system.into(),
            rules: rules.into(),
            candidates: candidates.into_iter().collect(),
        }
    }

    pub fn digest(&self) -> String {
        canonical_digest(self)
    }
}
