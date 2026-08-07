// Copyright 2026 Mark AB (markik)
// SPDX-License-Identifier: MIT OR Apache-2.0

use cleromancy::{Candidate, CleromancyHost, ContextSnapshot, Field, ReadingEngine};
use graphshell_endpoint::{PresentationSource, ProjectionSource};
use graphshell_protocol::{
    Carrier, CarrierRequestBody, CarrierResponseBody, EndpointDescriptor, ProjectionOffer,
    ProjectionSession, ResourceRequest,
};
use muniment::{Backend, MemoryBackend};

pub fn fixture_carrier() -> FixtureCarrier {
    let context = ContextSnapshot::new("Field notes", "fixture.external-context/v1")
        .with_fact("browsing", "Radio harmony")
        .with_tags(["field", "notes"]);
    let field = Field::new(
        "fixture.external-system/v1",
        "contextual-weight/v1",
        [Candidate::new(
            "signal",
            "A remote signal",
            "The endpoint retains this reading.",
        )],
    );
    let reading = ReadingEngine::calculate(&context, &field).unwrap();
    let mut host = CleromancyHost::empty(MemoryBackend::new());
    host.insert_reading(&context, &field, &reading).unwrap();
    FixtureCarrier {
        host,
        session: ProjectionSession("external:fixture".to_string()),
        closed: false,
    }
}

pub fn truth_bytes<B: Backend>(host: &CleromancyHost<B>) -> Vec<u8> {
    serde_json::to_vec(&(host.graph().to_snapshot(), host.graph().facets())).unwrap()
}

pub struct FixtureCarrier {
    host: CleromancyHost<MemoryBackend>,
    session: ProjectionSession,
    closed: bool,
}

impl Carrier for FixtureCarrier {
    fn request(
        &mut self,
        body: CarrierRequestBody,
    ) -> Result<CarrierResponseBody, graphshell_protocol::CarrierError> {
        if self.closed {
            // A closed carrier is gone, not declining.
            return Err(graphshell_protocol::CarrierError::Disconnected(
                "fixture carrier is closed".to_string(),
            ));
        }
        match body {
            CarrierRequestBody::Discover => {
                let local = self.host.local_request();
                Ok(CarrierResponseBody::Descriptor(EndpointDescriptor {
                    label: "Fixture source".to_string(),
                    projections: vec![ProjectionOffer {
                        label: "Fixture graph".to_string(),
                        request: graphshell_protocol::ProjectionRequest {
                            version: local.version,
                            session: self.session.clone(),
                            score: local.score,
                        },
                    }],
                }))
            }
            CarrierRequestBody::Snapshot(request) if request.session == self.session => {
                let mut local = request;
                local.session = self.host.session();
                let mut snapshot = self
                    .host
                    .snapshot(local)
                    .map_err(|error| {
                        graphshell_protocol::CarrierError::Refused(error.to_string())
                    })?;
                snapshot.session = self.session.clone();
                Ok(CarrierResponseBody::Snapshot(Box::new(snapshot)))
            }
            CarrierRequestBody::Resource(request) if request.session == self.session => {
                let response = self
                    .host
                    .resource(ResourceRequest {
                        session: self.host.session(),
                        resource: request.resource,
                    })
                    .map_err(|error| {
                        graphshell_protocol::CarrierError::Refused(error.to_string())
                    })?;
                Ok(CarrierResponseBody::Resource(
                    graphshell_protocol::ResourceResponse {
                        session: self.session.clone(),
                        resource: response.resource,
                        bytes: response.bytes,
                    },
                ))
            }
            CarrierRequestBody::Close => {
                self.closed = true;
                Ok(CarrierResponseBody::Closed)
            }
            _ => Err(graphshell_protocol::CarrierError::Refused(
                "fixture carrier refused the request".to_string(),
            )),
        }
    }

    fn take_notice(&mut self) -> Option<graphshell_protocol::CarrierNotice> {
        None
    }

    fn wait_for_notice(
        &mut self,
    ) -> Result<graphshell_protocol::CarrierNotice, graphshell_protocol::CarrierError> {
        Err(graphshell_protocol::CarrierError::Refused(
            "fixture emits no notices".to_string(),
        ))
    }

    fn shutdown(&mut self) -> Result<(), graphshell_protocol::CarrierError> {
        self.closed = true;
        Ok(())
    }
}
