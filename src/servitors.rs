// Copyright 2026 Mark AB (markik)
// SPDX-License-Identifier: MIT OR Apache-2.0

use chartulary::{Container, EditSpec, GraphLog, Relation};
use servitor::{
    AuthorityProvider, Cap, CapError, Gate, Grant, GrantTable, Mode, ScopePath, Subject,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ServitorAccessError {
    #[error("invalid Servitor scope: {0}")]
    Scope(#[from] CapError),
    #[error("Servitor gate: {0}")]
    Gate(String),
}

/// The explicit application seam into Servitor. A0 exposes the real gate and
/// authority table without claiming that its nested-graph mutation path is a
/// Cleromancy reading operation.
pub struct ServitorAccess {
    gate: Gate,
    authority: GrantTable,
    audit: GraphLog<Container, Relation>,
}

impl Default for ServitorAccess {
    fn default() -> Self {
        Self {
            gate: Gate::new(),
            authority: GrantTable::new(),
            audit: GraphLog::new(),
        }
    }
}

impl ServitorAccess {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn gate(&self) -> &Gate {
        &self.gate
    }

    pub fn authority(&self) -> &GrantTable {
        &self.authority
    }

    pub fn authority_mut(&mut self) -> &mut GrantTable {
        &mut self.authority
    }

    pub fn grant(&mut self, grant: Grant) -> Result<(), ServitorAccessError> {
        self.gate
            .project_grant(&mut self.audit, &grant)
            .map_err(|error| ServitorAccessError::Gate(format!("{error:?}")))?;
        self.authority.grant(grant);
        Ok(())
    }

    pub fn allows(&self, subject: Subject, needed: &Cap, mode: Mode) -> bool {
        self.authority.covers(subject, needed, mode)
    }

    /// Authorize and attribute one bound-session request through Servitor's
    /// real petition path. The audit graph records authorization, while the
    /// Cleromancy reading graph records the resulting domain receipt.
    pub fn petition_write(
        &mut self,
        subject: Subject,
        scope: &str,
        label: impl Into<String>,
    ) -> Result<(), ServitorAccessError> {
        let claimed = ScopePath::parse(scope)?;
        let revision = self.audit.revision();
        let node = Container::new(format!("{scope}/request-{revision}")).with_title(label);
        self.gate
            .petition(
                &self.authority,
                &mut self.audit,
                subject,
                &claimed,
                revision,
                vec![EditSpec::InsertNode(node)],
            )
            .map_err(|error| ServitorAccessError::Gate(format!("{error:?}")))?;
        Ok(())
    }

    pub fn audit(&self) -> &GraphLog<Container, Relation> {
        &self.audit
    }
}
