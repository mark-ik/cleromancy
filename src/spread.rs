// Copyright 2026 Mark AB (markik)
// SPDX-License-Identifier: MIT OR Apache-2.0

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::context::canonical_digest;
use crate::session::ReadingPlacement;

pub const THREE_CARD_SPREAD_SCHEMA: &str = "cleromancy.three-card-spread/v1";

/// The one authored layout in A8. These names are deliberately concrete: the
/// crate exposes a useful spread, not a general spread language.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThreeCardPosition {
    Foundation,
    Tension,
    NextStep,
}

impl ThreeCardPosition {
    pub const ALL: [Self; 3] = [Self::Foundation, Self::Tension, Self::NextStep];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Foundation => "foundation",
            Self::Tension => "tension",
            Self::NextStep => "next_step",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThreeCardRelationKind {
    Questions,
    NextStep,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThreeCardPlacement {
    pub position: ThreeCardPosition,
    pub reading_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThreeCardRelation {
    pub from: ThreeCardPosition,
    pub to: ThreeCardPosition,
    pub kind: ThreeCardRelationKind,
    pub label: String,
}

/// An authored three-card interpretation frame attached to a saved session.
/// The session remains the reusable ordered record; this node adds the
/// position names and the two explicit relationships shown in the graph.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThreeCardSpread {
    pub schema: String,
    pub id: String,
    pub session_id: String,
    pub placements: Vec<ThreeCardPlacement>,
    pub relations: Vec<ThreeCardRelation>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SpreadError {
    #[error("three-card spread is invalid: {0}")]
    InvalidSpread(String),
}

impl ThreeCardSpread {
    pub fn new(
        session_id: impl Into<String>,
        session_placements: &[ReadingPlacement],
    ) -> Result<Self, SpreadError> {
        let session_id = session_id.into();
        let placements = ThreeCardPosition::ALL
            .into_iter()
            .map(|position| {
                let reading_id = session_placements
                    .iter()
                    .find(|placement| placement.position == position.as_str())
                    .map(|placement| placement.reading_id.clone())
                    .ok_or_else(|| SpreadError::InvalidSpread("session positions".to_string()))?;
                Ok(ThreeCardPlacement {
                    position,
                    reading_id,
                })
            })
            .collect::<Result<Vec<_>, SpreadError>>()?;
        let relations = vec![
            ThreeCardRelation {
                from: ThreeCardPosition::Tension,
                to: ThreeCardPosition::Foundation,
                kind: ThreeCardRelationKind::Questions,
                label: "tests the foundation".to_string(),
            },
            ThreeCardRelation {
                from: ThreeCardPosition::NextStep,
                to: ThreeCardPosition::Tension,
                kind: ThreeCardRelationKind::NextStep,
                label: "answers the tension".to_string(),
            },
        ];
        let id = three_card_spread_id(&session_id, &placements, &relations);
        let spread = Self {
            schema: THREE_CARD_SPREAD_SCHEMA.to_string(),
            id,
            session_id,
            placements,
            relations,
        };
        spread.validate()?;
        Ok(spread)
    }

    pub fn validate(&self) -> Result<(), SpreadError> {
        if self.schema != THREE_CARD_SPREAD_SCHEMA {
            return Err(SpreadError::InvalidSpread("schema".to_string()));
        }
        if !is_digest(&self.session_id) {
            return Err(SpreadError::InvalidSpread("session id".to_string()));
        }
        if self.placements.len() != 3
            || self
                .placements
                .iter()
                .map(|placement| placement.position)
                .collect::<Vec<_>>()
                != ThreeCardPosition::ALL
        {
            return Err(SpreadError::InvalidSpread("placements".to_string()));
        }
        for placement in &self.placements {
            if !is_digest(&placement.reading_id) {
                return Err(SpreadError::InvalidSpread(
                    "placement reading id".to_string(),
                ));
            }
        }
        let expected_relations = vec![
            ThreeCardRelation {
                from: ThreeCardPosition::Tension,
                to: ThreeCardPosition::Foundation,
                kind: ThreeCardRelationKind::Questions,
                label: "tests the foundation".to_string(),
            },
            ThreeCardRelation {
                from: ThreeCardPosition::NextStep,
                to: ThreeCardPosition::Tension,
                kind: ThreeCardRelationKind::NextStep,
                label: "answers the tension".to_string(),
            },
        ];
        if self.relations != expected_relations {
            return Err(SpreadError::InvalidSpread("authored relations".to_string()));
        }
        if self.id != three_card_spread_id(&self.session_id, &self.placements, &self.relations) {
            return Err(SpreadError::InvalidSpread("identity".to_string()));
        }
        Ok(())
    }
}

#[derive(Serialize)]
struct ThreeCardSpreadIdentity<'a> {
    schema: &'static str,
    session_id: &'a str,
    placements: &'a [ThreeCardPlacement],
    relations: &'a [ThreeCardRelation],
}

fn three_card_spread_id(
    session_id: &str,
    placements: &[ThreeCardPlacement],
    relations: &[ThreeCardRelation],
) -> String {
    canonical_digest(&ThreeCardSpreadIdentity {
        schema: THREE_CARD_SPREAD_SCHEMA,
        session_id,
        placements,
        relations,
    })
}

fn is_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}
