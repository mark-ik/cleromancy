// Copyright 2026 Mark AB (markik)
// SPDX-License-Identifier: MIT OR Apache-2.0

use graphshell_endpoint::{IntentSink, PresentationSource, ProjectionCatalog, ProjectionSource};
use graphshell_protocol::{
    EndpointDescriptor, IntentInvocation, IntentResult, ProjectionOffer, ProjectionRequest,
    ProjectionSnapshot, ProtocolVersion, ResourceRequest, ResourceResponse,
};
use muniment::Backend;

use crate::host::{CleromancyHost, HostError};

impl<B: Backend> ProjectionCatalog for CleromancyHost<B> {
    fn describe(&self) -> EndpointDescriptor {
        EndpointDescriptor {
            label: "Local Cleromancy readings".to_string(),
            projections: vec![ProjectionOffer {
                label: "Current readings".to_string(),
                request: self.local_request(),
            }],
        }
    }
}

impl<B: Backend> ProjectionSource for CleromancyHost<B> {
    type Error = HostError;

    fn snapshot(&mut self, request: ProjectionRequest) -> Result<ProjectionSnapshot, Self::Error> {
        if request.session != self.session() || request.version.major != ProtocolVersion::V1.major {
            return Err(HostError::WrongSession);
        }
        self.build_snapshot()
    }
}

impl<B: Backend> PresentationSource for CleromancyHost<B> {
    type Error = HostError;

    fn resource(&mut self, request: ResourceRequest) -> Result<ResourceResponse, Self::Error> {
        if request.session != self.session() {
            return Err(HostError::WrongSession);
        }
        let bytes = self
            .resources
            .get(&request.resource)
            .cloned()
            .ok_or(HostError::MissingResource)?;
        Ok(ResourceResponse {
            session: request.session,
            resource: request.resource,
            bytes,
        })
    }
}

impl<B: Backend> IntentSink for CleromancyHost<B> {
    type Error = HostError;

    fn invoke(&mut self, intent: IntentInvocation) -> Result<IntentResult, Self::Error> {
        if intent.session != self.session() {
            return Err(HostError::WrongSession);
        }
        Ok(IntentResult::Rejected {
            reason: "A0 exposes readings as a read-only Graphshell projection".to_string(),
        })
    }
}
