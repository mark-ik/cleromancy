// Copyright 2026 Mark AB (markik)
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Cleromancy's adapter for a Graphshell session already admitted by its host.
//!
//! The surrounding Graphshell session loop retains the delegation chain and
//! checks expiry and revocation. This adapter receives only the projected
//! session name and subject, which it uses to scope Cleromancy's own endpoint
//! and Servitor petitions. It does not authenticate a new caller.

use std::sync::{Arc, Mutex};

use graphshell::lifecycle::{AdmittedEndpointContext, BindAdmittedSession};
use graphshell::native::endpoint_catalog::{ResidentEndpointCatalog, ResidentEndpointCatalogError};
use graphshell_endpoint::{
    IntentSink, PresentationSource, ProjectionCatalog, ProjectionNoticeSource, ProjectionSource,
};
use graphshell_protocol::{
    CarrierNotice, EndpointDescriptor, IntentInvocation, IntentResult, ProjectionRequest,
    ProjectionSnapshot, ResourceRequest, ResourceResponse,
};
use muniment::Backend;
use servitor::Subject;

use crate::app::CleromancyAppSessionState;
use crate::{AppError, CleromancyApp};

impl<B: Backend> BindAdmittedSession for CleromancyApp<B> {
    fn bind_admitted_session(mut self, context: &AdmittedEndpointContext) -> Self {
        self.host.bind_projection_session(context.session().clone());
        self.bind_intent_subject(Subject::new(context.subject()));
        self
    }
}

/// One durable Cleromancy graph and Servitor authority shared by resident
/// endpoints. Each endpoint receives its own projection state at open time.
pub struct CleromancySessionAuthority<B> {
    app: Arc<Mutex<CleromancyApp<B>>>,
}

impl<B> Clone for CleromancySessionAuthority<B> {
    fn clone(&self) -> Self {
        Self {
            app: Arc::clone(&self.app),
        }
    }
}

impl<B: Backend + Send + 'static> CleromancySessionAuthority<B> {
    pub fn new(app: CleromancyApp<B>) -> Self {
        Self {
            app: Arc::new(Mutex::new(app)),
        }
    }

    pub fn endpoint(&self, context: &AdmittedEndpointContext) -> CleromancySessionEndpoint<B> {
        CleromancySessionEndpoint {
            authority: self.clone(),
            state: Mutex::new(ResidentSessionState {
                app: CleromancyAppSessionState::admitted(
                    context.session().clone(),
                    Subject::new(context.subject()),
                ),
                last_notified: None,
            }),
        }
    }

    /// Register a factory that gives every already-admitted session a fresh
    /// endpoint over this authority's durable reading graph.
    pub fn register_catalog(
        &self,
        catalog: &mut ResidentEndpointCatalog,
        id: impl Into<String>,
        label: impl Into<String>,
    ) -> Result<(), ResidentEndpointCatalogError> {
        let authority = self.clone();
        catalog.register_notifying(id, label, move |context| Ok(authority.endpoint(context)))
    }
}

/// The session-local endpoint selected by a Graphshell resident catalog.
///
/// Its state owns only projection ephemera and the Servitor subject. The
/// shared authority serializes mutations of Cleromancy's durable graph.
pub struct CleromancySessionEndpoint<B> {
    authority: CleromancySessionAuthority<B>,
    state: Mutex<ResidentSessionState>,
}

struct ResidentSessionState {
    app: CleromancyAppSessionState,
    last_notified: Option<(scenotime::SceneEpoch, scenotime::Revision)>,
}

impl<B: Backend + Send + 'static> CleromancySessionEndpoint<B> {
    fn with_app<R>(&self, operation: impl FnOnce(&mut CleromancyApp<B>) -> R) -> R {
        let mut app = self
            .authority
            .app
            .lock()
            .expect("Cleromancy resident authority lock poisoned");
        let mut state = self
            .state
            .lock()
            .expect("Cleromancy resident endpoint lock poisoned");
        app.with_session_state(&mut state.app, operation)
    }

    fn record_snapshot(&self, revision: Option<(scenotime::SceneEpoch, scenotime::Revision)>) {
        if let Some(revision) = revision {
            self.state
                .lock()
                .expect("Cleromancy resident endpoint lock poisoned")
                .last_notified = Some(revision);
        }
    }
}

impl<B: Backend + Send + 'static> ProjectionCatalog for CleromancySessionEndpoint<B> {
    fn describe(&self) -> EndpointDescriptor {
        self.with_app(|app| app.describe())
    }
}

impl<B: Backend + Send + 'static> ProjectionSource for CleromancySessionEndpoint<B> {
    type Error = AppError;

    fn snapshot(&mut self, request: ProjectionRequest) -> Result<ProjectionSnapshot, Self::Error> {
        let (snapshot, revision) = self.with_app(|app| {
            let snapshot = app.snapshot(request);
            let revision = app.host.last_snapshot_revision();
            (snapshot, revision)
        });
        if snapshot.is_ok() {
            self.record_snapshot(revision);
        }
        snapshot
    }
}

impl<B: Backend + Send + 'static> PresentationSource for CleromancySessionEndpoint<B> {
    type Error = AppError;

    fn resource(&mut self, request: ResourceRequest) -> Result<ResourceResponse, Self::Error> {
        self.with_app(|app| app.resource(request))
    }
}

impl<B: Backend + Send + 'static> IntentSink for CleromancySessionEndpoint<B> {
    type Error = AppError;

    fn invoke(&mut self, intent: IntentInvocation) -> Result<IntentResult, Self::Error> {
        self.with_app(|app| app.invoke(intent))
    }
}

impl<B: Backend + Send + 'static> ProjectionNoticeSource for CleromancySessionEndpoint<B> {
    type Error = AppError;

    fn poll_notice(&mut self) -> Result<Option<CarrierNotice>, Self::Error> {
        let (observed, current, session) = self.with_app(|app| {
            (
                app.host.last_snapshot_revision(),
                app.host.current_projection_revision(),
                app.host.session(),
            )
        });
        let Some(observed) = observed else {
            return Ok(None);
        };
        if current <= observed {
            return Ok(None);
        }

        let mut state = self
            .state
            .lock()
            .expect("Cleromancy resident endpoint lock poisoned");
        if state.last_notified == Some(current) {
            return Ok(None);
        }
        state.last_notified = Some(current);
        Ok(Some(CarrierNotice {
            session,
            epoch: current.0,
            revision: current.1,
        }))
    }
}
