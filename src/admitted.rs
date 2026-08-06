// Copyright 2026 Mark AB (markik)
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Cleromancy's adapter for a Graphshell session already admitted by its host.
//!
//! The surrounding Graphshell session loop retains the delegation chain and
//! checks expiry and revocation. This adapter receives only the projected
//! session name and subject, which it uses to scope Cleromancy's own endpoint
//! and Servitor petitions. It does not authenticate a new caller.

use graphshell::lifecycle::{AdmittedEndpointContext, BindAdmittedSession};
use muniment::Backend;
use servitor::Subject;

use crate::CleromancyApp;

impl<B: Backend> BindAdmittedSession for CleromancyApp<B> {
    fn bind_admitted_session(mut self, context: &AdmittedEndpointContext) -> Self {
        self.host.bind_projection_session(context.session().clone());
        self.bind_intent_subject(Subject::new(context.subject()));
        self
    }
}
