// Copyright 2026 Mark AB (markik)
// SPDX-License-Identifier: MIT OR Apache-2.0

use graphshell_protocol::{AdvertisedAction, IntentEffect, IntentReference};
use serde::{Deserialize, Serialize};

use crate::{Candidate, Field, SealedEnrichment, SelectionMode, UNIFORM_DIE_RULE};

pub const READ_INTENT: &str = "cleromancy.read";
pub const SELECT_INTENT: &str = "cleromancy.select";
pub const ROLL_INTENT: &str = "cleromancy.roll";
pub const THREE_CARD_SPREAD_INTENT: &str = "cleromancy.three-card-spread";
pub const COMPOSE_READING_INTENT: &str = "cleromancy.compose-reading";
pub const CREATE_CONCURRENCE_INTENT: &str = "cleromancy.create-concurrence";
pub const READ_SCHEMA: &str = "cleromancy.intent.read/v1";
pub const SELECT_SCHEMA: &str = "cleromancy.intent.select/v1";
pub const ROLL_SCHEMA: &str = "cleromancy.intent.roll/v1";
pub const THREE_CARD_SPREAD_INTENT_SCHEMA: &str = "cleromancy.intent.three-card-spread/v1";
pub const COMPOSE_READING_SCHEMA: &str = "cleromancy.intent.compose-reading/v2";
pub const CREATE_CONCURRENCE_SCHEMA: &str = "cleromancy.intent.create-concurrence/v1";
pub const READ_SCOPE: &str = "cleromancy/intents/read";
pub const SELECT_SCOPE: &str = "cleromancy/intents/select";
pub const ROLL_SCOPE: &str = "cleromancy/intents/roll";
pub const THREE_CARD_SPREAD_SCOPE: &str = "cleromancy/intents/three-card-spread";
pub const COMPOSE_READING_SCOPE: &str = "cleromancy/intents/compose-reading";
pub const CREATE_CONCURRENCE_SCOPE: &str = "cleromancy/intents/create-concurrence";

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

/// The product-neutral composition selected by a headed field composer.
///
/// The field is always explicit and retained by the host after acceptance.
/// Layout and selection mode are separate so a caller can use the same
/// composer for a deterministic consultation, a single cast, or the authored
/// three-card cast. Unsupported combinations are rejected by the app rather
/// than silently changing the requested reading.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompositionLayout {
    Single,
    ThreeCard,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum FieldSelection {
    Inline { field: Field },
    Stored { digest: String },
}

impl FieldSelection {
    pub fn inline(field: Field) -> Self {
        Self::Inline { field }
    }

    pub fn stored(digest: impl Into<String>) -> Self {
        Self::Stored {
            digest: digest.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadingCompositionIntentPayload {
    pub schema: String,
    pub field: FieldSelection,
    pub layout: CompositionLayout,
    pub mode: SelectionMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enrichment: Option<SealedEnrichment>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_token: Option<String>,
}

/// One explicit pairing selected from saved graph truth. The target card must
/// name either the supplied facts digest or reading session ID, so a caller
/// cannot present an unrelated card as the selected occasion member.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AstrologyReadingConcurrenceIntentPayload {
    pub schema: String,
    pub astrology_facts_digest: String,
    pub reading_session_id: String,
}

impl AstrologyReadingConcurrenceIntentPayload {
    pub fn new(
        astrology_facts_digest: impl Into<String>,
        reading_session_id: impl Into<String>,
    ) -> Self {
        Self {
            schema: CREATE_CONCURRENCE_SCHEMA.to_string(),
            astrology_facts_digest: astrology_facts_digest.into(),
            reading_session_id: reading_session_id.into(),
        }
    }
}

impl ReadingCompositionIntentPayload {
    pub fn new(field: Field, layout: CompositionLayout, mode: SelectionMode) -> Self {
        Self {
            schema: COMPOSE_READING_SCHEMA.to_string(),
            field: FieldSelection::inline(field),
            layout,
            mode,
            enrichment: None,
            client_token: None,
        }
    }

    pub fn stored(
        digest: impl Into<String>,
        layout: CompositionLayout,
        mode: SelectionMode,
    ) -> Self {
        Self {
            schema: COMPOSE_READING_SCHEMA.to_string(),
            field: FieldSelection::stored(digest),
            layout,
            mode,
            enrichment: None,
            client_token: None,
        }
    }

    pub fn single_calculated(field: Field) -> Self {
        Self::new(field, CompositionLayout::Single, SelectionMode::Calculated)
    }

    pub fn single_cast(field: Field) -> Self {
        Self::new(field, CompositionLayout::Single, SelectionMode::Cast)
    }

    pub fn three_card(field: Field) -> Self {
        Self::new(field, CompositionLayout::ThreeCard, SelectionMode::Cast)
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
        AdvertisedAction {
            intent: IntentReference(COMPOSE_READING_INTENT.to_string()),
            label: "Compose a reading".to_string(),
            explanation:
                "Choose an explicit field, layout, and deterministic or cast mode; the selected composition is saved with its workings."
                    .to_string(),
            payload_schema: COMPOSE_READING_SCHEMA.to_string(),
            effect: IntentEffect::DomainTruth,
        },
    ]
}

pub fn concurrence_actions() -> Vec<AdvertisedAction> {
    vec![AdvertisedAction {
        intent: IntentReference(CREATE_CONCURRENCE_INTENT.to_string()),
        label: "Save pattern occasion".to_string(),
        explanation:
            "Pair this saved astrology fact set or reading session with its selected counterpart. The result records only that they were consulted together."
                .to_string(),
        payload_schema: CREATE_CONCURRENCE_SCHEMA.to_string(),
        effect: IntentEffect::DomainTruth,
    }]
}

pub(crate) fn scope_for(intent: &str) -> Option<&'static str> {
    match intent {
        READ_INTENT => Some(READ_SCOPE),
        SELECT_INTENT => Some(SELECT_SCOPE),
        ROLL_INTENT => Some(ROLL_SCOPE),
        THREE_CARD_SPREAD_INTENT => Some(THREE_CARD_SPREAD_SCOPE),
        COMPOSE_READING_INTENT => Some(COMPOSE_READING_SCOPE),
        CREATE_CONCURRENCE_INTENT => Some(CREATE_CONCURRENCE_SCOPE),
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
