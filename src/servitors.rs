// Copyright 2026 Mark AB (markik)
// SPDX-License-Identifier: MIT OR Apache-2.0

use servitor::{AuthorityProvider, Cap, Gate, Grant, GrantTable, Mode, Subject};

/// The explicit application seam into Servitor. A0 exposes the real gate and
/// authority table without claiming that its nested-graph mutation path is a
/// Cleromancy reading operation.
#[derive(Clone, Debug, Default)]
pub struct ServitorAccess {
    gate: Gate,
    authority: GrantTable,
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

    pub fn grant(&mut self, grant: Grant) {
        self.authority.grant(grant);
    }

    pub fn allows(&self, subject: Subject, needed: &Cap, mode: Mode) -> bool {
        self.authority.covers(subject, needed, mode)
    }
}
