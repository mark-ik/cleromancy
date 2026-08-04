// Copyright 2026 Mark AB (markik)
// SPDX-License-Identifier: MIT OR Apache-2.0

use graphshell_endpoint::{
    IntentSink, PresentationSource, ProjectionCatalog, ProjectionNoticeSource, ProjectionSource,
};
use graphshell_protocol::{
    CarrierNotice, EndpointDescriptor, IntentInvocation, IntentResult, ProjectionOffer,
    ProjectionRequest, ProjectionSnapshot, ProtocolVersion, ResourceRequest, ResourceResponse,
};
use muniment::Backend;

use crate::host::{CleromancyHost, HostError};
use crate::{AppError, CleromancyApp};

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
            reason: "The raw host projection is read-only; bound intents require CleromancyApp"
                .to_string(),
        })
    }
}

impl<B: Backend> ProjectionCatalog for CleromancyApp<B> {
    fn describe(&self) -> EndpointDescriptor {
        self.host.describe()
    }
}

impl<B: Backend> ProjectionSource for CleromancyApp<B> {
    type Error = AppError;

    fn snapshot(&mut self, request: ProjectionRequest) -> Result<ProjectionSnapshot, Self::Error> {
        if request.session != self.host.session()
            || request.version.major != ProtocolVersion::V1.major
        {
            return Err(HostError::WrongSession.into());
        }
        Ok(self
            .host
            .build_snapshot_with_actions(self.intents_are_bound())?)
    }
}

impl<B: Backend> PresentationSource for CleromancyApp<B> {
    type Error = AppError;

    fn resource(&mut self, request: ResourceRequest) -> Result<ResourceResponse, Self::Error> {
        Ok(self.host.resource(request)?)
    }
}

impl<B: Backend> IntentSink for CleromancyApp<B> {
    type Error = AppError;

    fn invoke(&mut self, intent: IntentInvocation) -> Result<IntentResult, Self::Error> {
        CleromancyApp::invoke(self, intent)
    }
}

impl<B: Backend> ProjectionNoticeSource for CleromancyApp<B> {
    type Error = AppError;

    fn poll_notice(&mut self) -> Result<Option<CarrierNotice>, Self::Error> {
        Ok(self.take_projection_notice())
    }
}
