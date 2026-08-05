// Copyright 2026 Mark AB (markik)
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::context::canonical_digest;

pub const CONCURRENCE_SCHEMA: &str = "cleromancy.concurrence/v1";
pub const ASTROLOGY_FACTS_ROLE: &str = "astrology-facts";
pub const READING_SESSION_ROLE: &str = "reading-session";

/// One graph-resident value consulted during the same saved occasion. A role
/// describes participation in the occasion, not influence or causation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConcurrenceMember {
    pub role: String,
    pub address: String,
}

impl ConcurrenceMember {
    pub fn new(role: impl Into<String>, address: impl Into<String>) -> Self {
        Self {
            role: role.into(),
            address: address.into(),
        }
    }
}

/// An immutable grouping of independently produced graph values which were
/// consulted together. It records concurrence without asserting that one
/// member qualified, explained, or caused another.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Concurrence {
    pub schema: String,
    pub id: String,
    pub created_at_ms: u64,
    pub label: String,
    pub members: Vec<ConcurrenceMember>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ConcurrenceError {
    #[error("concurrence is invalid: {0}")]
    Invalid(String),
}

impl Concurrence {
    pub fn new(
        created_at_ms: u64,
        label: impl Into<String>,
        members: impl IntoIterator<Item = ConcurrenceMember>,
    ) -> Result<Self, ConcurrenceError> {
        let mut members = members.into_iter().collect::<Vec<_>>();
        members.sort_by(|left, right| {
            left.address
                .cmp(&right.address)
                .then_with(|| left.role.cmp(&right.role))
        });
        let label = label.into();
        let id = concurrence_id(created_at_ms, &label, &members);
        let concurrence = Self {
            schema: CONCURRENCE_SCHEMA.to_string(),
            id,
            created_at_ms,
            label,
            members,
        };
        concurrence.validate()?;
        Ok(concurrence)
    }

    pub fn astrology_reading(
        created_at_ms: u64,
        astrology_facts_digest: &str,
        reading_session_id: &str,
    ) -> Result<Self, ConcurrenceError> {
        if !is_digest(astrology_facts_digest) {
            return Err(ConcurrenceError::Invalid(
                "astrology facts digest".to_string(),
            ));
        }
        if !is_digest(reading_session_id) {
            return Err(ConcurrenceError::Invalid("reading session id".to_string()));
        }
        Self::new(
            created_at_ms,
            "Astrology and reading",
            [
                ConcurrenceMember::new(
                    ASTROLOGY_FACTS_ROLE,
                    format!("cleromancy://astrology/facts/{astrology_facts_digest}"),
                ),
                ConcurrenceMember::new(
                    READING_SESSION_ROLE,
                    format!("cleromancy://session/{reading_session_id}"),
                ),
            ],
        )
    }

    pub fn validate(&self) -> Result<(), ConcurrenceError> {
        invalid_if(self.schema != CONCURRENCE_SCHEMA, "schema")?;
        invalid_if(
            self.label.trim().is_empty() || self.label.len() > 256,
            "label",
        )?;
        invalid_if(
            self.members.len() < 2 || self.members.len() > 64,
            "member count",
        )?;
        let mut addresses = BTreeSet::new();
        for member in &self.members {
            invalid_if(!valid_role(&member.role), "member role")?;
            invalid_if(!valid_address(&member.address), "member address")?;
            invalid_if(
                !addresses.insert(member.address.as_str()),
                "duplicate member",
            )?;
        }
        invalid_if(
            self.members.windows(2).any(|pair| {
                (pair[0].address.as_str(), pair[0].role.as_str())
                    > (pair[1].address.as_str(), pair[1].role.as_str())
            }),
            "member order",
        )?;
        invalid_if(
            self.id != concurrence_id(self.created_at_ms, &self.label, &self.members),
            "identity",
        )?;
        Ok(())
    }
}

#[derive(Serialize)]
struct ConcurrenceIdentity<'a> {
    schema: &'static str,
    created_at_ms: u64,
    label: &'a str,
    members: &'a [ConcurrenceMember],
}

fn concurrence_id(created_at_ms: u64, label: &str, members: &[ConcurrenceMember]) -> String {
    canonical_digest(&ConcurrenceIdentity {
        schema: CONCURRENCE_SCHEMA,
        created_at_ms,
        label,
        members,
    })
}

fn valid_role(role: &str) -> bool {
    !role.is_empty()
        && role.len() <= 64
        && role
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn valid_address(address: &str) -> bool {
    address.starts_with("cleromancy://")
        && address.len() > "cleromancy://".len()
        && address.len() <= 2048
        && !address.chars().any(char::is_whitespace)
}

fn is_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn invalid_if(condition: bool, field: &str) -> Result<(), ConcurrenceError> {
    if condition {
        return Err(ConcurrenceError::Invalid(field.to_string()));
    }
    Ok(())
}
