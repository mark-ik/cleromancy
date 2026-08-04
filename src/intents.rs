// Copyright 2026 Mark AB (markik)
// SPDX-License-Identifier: MIT OR Apache-2.0

use graphshell_protocol::{AdvertisedAction, IntentEffect, IntentReference};
use serde::{Deserialize, Serialize};

use crate::{Candidate, Field, SealedEnrichment, UNIFORM_DIE_RULE};

pub const READ_INTENT: &str = "cleromancy.read";
pub const SELECT_INTENT: &str = "cleromancy.select";
pub const ROLL_INTENT: &str = "cleromancy.roll";
pub const THREE_CARD_SPREAD_INTENT: &str = "cleromancy.three-card-spread";
pub const READ_SCHEMA: &str = "cleromancy.intent.read/v1";
pub const SELECT_SCHEMA: &str = "cleromancy.intent.select/v1";
pub const ROLL_SCHEMA: &str = "cleromancy.intent.roll/v1";
pub const THREE_CARD_SPREAD_INTENT_SCHEMA: &str = "cleromancy.intent.three-card-spread/v1";
pub const READ_SCOPE: &str = "cleromancy/intents/read";
pub const SELECT_SCOPE: &str = "cleromancy/intents/select";
pub const ROLL_SCOPE: &str = "cleromancy/intents/roll";
pub const THREE_CARD_SPREAD_SCOPE: &str = "cleromancy/intents/three-card-spread";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IntentLimits {
    pub max_payload_bytes: usize,
    pub max_candidates: usize,
    pub max_die_sides: u32,
    pub max_client_token_bytes: usize,
}

impl Default for IntentLimits {
    fn default() -> Self {
        Self {
            max_payload_bytes: 64 * 1024,
            max_candidates: 512,
            max_die_sides: 1_000,
            max_client_token_bytes: 256,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadingIntentPayload {
    pub schema: String,
    pub field: Field,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enrichment: Option<SealedEnrichment>,
    /// Opaque caller correlation carried onto the saved session. An accepted
    /// intent still requires resnapshot before the caller can inspect it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_token: Option<String>,
}

impl ReadingIntentPayload {
    pub fn read(field: Field) -> Self {
        Self {
            schema: READ_SCHEMA.to_string(),
            field,
            enrichment: None,
            client_token: None,
        }
    }

    pub fn select(field: Field) -> Self {
        Self {
            schema: SELECT_SCHEMA.to_string(),
            field,
            enrichment: None,
            client_token: None,
        }
    }

    pub fn with_enrichment(mut self, evidence: SealedEnrichment) -> Self {
        self.enrichment = Some(evidence);
        self
    }

    pub fn with_client_token(mut self, client_token: impl Into<String>) -> Self {
        self.client_token = Some(client_token.into());
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RollIntentPayload {
    pub schema: String,
    pub sides: u32,
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_token: Option<String>,
}

impl RollIntentPayload {
    pub fn new(sides: u32) -> Self {
        Self {
            schema: ROLL_SCHEMA.to_string(),
            sides,
            label: None,
            client_token: None,
        }
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn with_client_token(mut self, client_token: impl Into<String>) -> Self {
        self.client_token = Some(client_token.into());
        self
    }

    pub fn field(&self) -> Field {
        die_field(self.sides, self.label.as_deref())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThreeCardSpreadIntentPayload {
    pub schema: String,
    pub field: Field,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_token: Option<String>,
}

impl ThreeCardSpreadIntentPayload {
    pub fn new(field: Field) -> Self {
        Self {
            schema: THREE_CARD_SPREAD_INTENT_SCHEMA.to_string(),
            field,
            client_token: None,
        }
    }

    pub fn with_client_token(mut self, client_token: impl Into<String>) -> Self {
        self.client_token = Some(client_token.into());
        self
    }
}

pub fn advertised_actions() -> Vec<AdvertisedAction> {
    vec![
        AdvertisedAction {
            intent: IntentReference(READ_INTENT.to_string()),
            label: "Read deterministically".to_string(),
            explanation: "Apply the declared qualifier and append a replayable reading."
                .to_string(),
            payload_schema: READ_SCHEMA.to_string(),
            effect: IntentEffect::DomainTruth,
        },
        AdvertisedAction {
            intent: IntentReference(SELECT_INTENT.to_string()),
            label: "Select with secure entropy".to_string(),
            explanation: "Cast across the qualified field and append its bounded sample receipt."
                .to_string(),
            payload_schema: SELECT_SCHEMA.to_string(),
            effect: IntentEffect::DomainTruth,
        },
        AdvertisedAction {
            intent: IntentReference(ROLL_INTENT.to_string()),
            label: "Roll a die".to_string(),
            explanation: "Cast one uniformly weighted die and append the replayable result."
                .to_string(),
            payload_schema: ROLL_SCHEMA.to_string(),
            effect: IntentEffect::DomainTruth,
        },
        AdvertisedAction {
            intent: IntentReference(THREE_CARD_SPREAD_INTENT.to_string()),
            label: "Cast a three-card spread".to_string(),
            explanation:
                "Cast foundation, tension, and next step, then append their replayable graph frame."
                    .to_string(),
            payload_schema: THREE_CARD_SPREAD_INTENT_SCHEMA.to_string(),
            effect: IntentEffect::DomainTruth,
        },
    ]
}

pub(crate) fn scope_for(intent: &str) -> Option<&'static str> {
    match intent {
        READ_INTENT => Some(READ_SCOPE),
        SELECT_INTENT => Some(SELECT_SCOPE),
        ROLL_INTENT => Some(ROLL_SCOPE),
        THREE_CARD_SPREAD_INTENT => Some(THREE_CARD_SPREAD_SCOPE),
        _ => None,
    }
}

pub(crate) fn die_field(sides: u32, label: Option<&str>) -> Field {
    let label = label
        .filter(|label| !label.trim().is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| format!("d{sides}"));
    Field::new(
        format!("cleromancy.die/d{sides}"),
        UNIFORM_DIE_RULE,
        (1..=sides).map(|face| {
            Candidate::new(
                face.to_string(),
                format!("{label}: {face}"),
                format!("A uniformly cast face of {label}."),
            )
        }),
    )
}
