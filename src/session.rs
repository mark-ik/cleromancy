// Copyright 2026 Mark AB (markik)
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::context::canonical_digest;

pub const READING_SESSION_SCHEMA: &str = "cleromancy.reading-session/v1";
pub const REFLECTION_SCHEMA: &str = "cleromancy.reflection/v1";

/// One ordered result in a reading occasion. A7 uses one `focus` placement;
/// the ordered form leaves a stable home for authored spreads without making a
/// spread language part of this slice.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadingPlacement {
    pub position: String,
    pub reading_id: String,
}

/// A saved occasion on which one or more sealed results were consulted. This
/// is deliberately distinct from [`crate::Reading`]: the same calculated
/// reading can belong to more than one occasion.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadingSession {
    pub schema: String,
    pub id: String,
    pub created_at_ms: u64,
    pub event_nonce: String,
    pub context_digest: String,
    pub field_digest: String,
    pub placements: Vec<ReadingPlacement>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_token: Option<String>,
}

/// A separately-addressed, immutable note about a reading occasion. Editing
/// later creates another reflection rather than rewriting either the sealed
/// result or the session that records it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reflection {
    pub schema: String,
    pub id: String,
    pub session_id: String,
    pub created_at_ms: u64,
    pub event_nonce: String,
    pub body: String,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SessionError {
    #[error("reading session is invalid: {0}")]
    InvalidSession(String),
    #[error("reflection is invalid: {0}")]
    InvalidReflection(String),
}

impl ReadingSession {
    pub fn single(
        created_at_ms: u64,
        event_nonce: impl Into<String>,
        context_digest: impl Into<String>,
        field_digest: impl Into<String>,
        reading_id: impl Into<String>,
        client_token: Option<String>,
    ) -> Result<Self, SessionError> {
        let event_nonce = event_nonce.into();
        let context_digest = context_digest.into();
        let field_digest = field_digest.into();
        let placements = vec![ReadingPlacement {
            position: "focus".to_string(),
            reading_id: reading_id.into(),
        }];
        let id = reading_session_id(
            created_at_ms,
            &event_nonce,
            &context_digest,
            &field_digest,
            &placements,
            client_token.as_deref(),
        );
        let session = Self {
            schema: READING_SESSION_SCHEMA.to_string(),
            id,
            created_at_ms,
            event_nonce,
            context_digest,
            field_digest,
            placements,
            client_token,
        };
        session.validate()?;
        Ok(session)
    }

    pub fn validate(&self) -> Result<(), SessionError> {
        if self.schema != READING_SESSION_SCHEMA {
            return Err(SessionError::InvalidSession("schema".to_string()));
        }
        if !is_digest(&self.context_digest) {
            return Err(SessionError::InvalidSession("context digest".to_string()));
        }
        if !is_digest(&self.field_digest) {
            return Err(SessionError::InvalidSession("field digest".to_string()));
        }
        if !is_nonce(&self.event_nonce) {
            return Err(SessionError::InvalidSession("event nonce".to_string()));
        }
        if self.placements.is_empty() {
            return Err(SessionError::InvalidSession("placements".to_string()));
        }
        let mut positions = BTreeSet::new();
        for placement in &self.placements {
            if placement.position.trim().is_empty() || placement.position.len() > 64 {
                return Err(SessionError::InvalidSession(
                    "placement position".to_string(),
                ));
            }
            if !positions.insert(&placement.position) {
                return Err(SessionError::InvalidSession(
                    "duplicate placement position".to_string(),
                ));
            }
            if !is_digest(&placement.reading_id) {
                return Err(SessionError::InvalidSession(
                    "placement reading id".to_string(),
                ));
            }
        }
        if self
            .client_token
            .as_ref()
            .is_some_and(|token| token.is_empty() || token.len() > 256)
        {
            return Err(SessionError::InvalidSession("client token".to_string()));
        }
        let expected = reading_session_id(
            self.created_at_ms,
            &self.event_nonce,
            &self.context_digest,
            &self.field_digest,
            &self.placements,
            self.client_token.as_deref(),
        );
        if self.id != expected {
            return Err(SessionError::InvalidSession("identity".to_string()));
        }
        Ok(())
    }
}

impl Reflection {
    pub fn new(
        session_id: impl Into<String>,
        created_at_ms: u64,
        event_nonce: impl Into<String>,
        body: impl Into<String>,
    ) -> Result<Self, SessionError> {
        let session_id = session_id.into();
        let event_nonce = event_nonce.into();
        let body = body.into();
        let id = reflection_id(&session_id, created_at_ms, &event_nonce, &body);
        let reflection = Self {
            schema: REFLECTION_SCHEMA.to_string(),
            id,
            session_id,
            created_at_ms,
            event_nonce,
            body,
        };
        reflection.validate()?;
        Ok(reflection)
    }

    pub fn validate(&self) -> Result<(), SessionError> {
        if self.schema != REFLECTION_SCHEMA {
            return Err(SessionError::InvalidReflection("schema".to_string()));
        }
        if !is_digest(&self.session_id) {
            return Err(SessionError::InvalidReflection("session id".to_string()));
        }
        if !is_nonce(&self.event_nonce) {
            return Err(SessionError::InvalidReflection("event nonce".to_string()));
        }
        if self.body.trim().is_empty() || self.body.len() > 64 * 1024 {
            return Err(SessionError::InvalidReflection("body".to_string()));
        }
        if self.id
            != reflection_id(
                &self.session_id,
                self.created_at_ms,
                &self.event_nonce,
                &self.body,
            )
        {
            return Err(SessionError::InvalidReflection("identity".to_string()));
        }
        Ok(())
    }
}

#[derive(Serialize)]
struct ReadingSessionIdentity<'a> {
    schema: &'static str,
    created_at_ms: u64,
    event_nonce: &'a str,
    context_digest: &'a str,
    field_digest: &'a str,
    placements: &'a [ReadingPlacement],
    client_token: Option<&'a str>,
}

fn reading_session_id(
    created_at_ms: u64,
    event_nonce: &str,
    context_digest: &str,
    field_digest: &str,
    placements: &[ReadingPlacement],
    client_token: Option<&str>,
) -> String {
    canonical_digest(&ReadingSessionIdentity {
        schema: READING_SESSION_SCHEMA,
        created_at_ms,
        event_nonce,
        context_digest,
        field_digest,
        placements,
        client_token,
    })
}

#[derive(Serialize)]
struct ReflectionIdentity<'a> {
    schema: &'static str,
    session_id: &'a str,
    created_at_ms: u64,
    event_nonce: &'a str,
    body: &'a str,
}

fn reflection_id(session_id: &str, created_at_ms: u64, event_nonce: &str, body: &str) -> String {
    canonical_digest(&ReflectionIdentity {
        schema: REFLECTION_SCHEMA,
        session_id,
        created_at_ms,
        event_nonce,
        body,
    })
}

fn is_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn is_nonce(value: &str) -> bool {
    value.len() == 32
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}
