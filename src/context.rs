// Copyright 2026 Mark AB (markik)
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

/// Versioned facts and tags from which a field is qualified.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextSnapshot {
    pub schema: String,
    pub label: String,
    pub facts: BTreeMap<String, String>,
    pub tags: BTreeSet<String>,
}

impl ContextSnapshot {
    pub fn new(label: impl Into<String>, schema: impl Into<String>) -> Self {
        Self {
            schema: schema.into(),
            label: label.into(),
            facts: BTreeMap::new(),
            tags: BTreeSet::new(),
        }
    }

    pub fn with_fact(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.facts.insert(name.into(), value.into());
        self
    }

    pub fn with_tags(mut self, tags: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.tags.extend(tags.into_iter().map(Into::into));
        self
    }

    pub fn digest(&self) -> String {
        canonical_digest(self)
    }
}

pub(crate) fn canonical_digest(value: &impl Serialize) -> String {
    let bytes = serde_json::to_vec(value).expect("versioned Cleromancy values always serialize");
    blake3::hash(&bytes).to_hex().to_string()
}
