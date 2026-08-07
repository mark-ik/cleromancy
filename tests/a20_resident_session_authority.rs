// Copyright 2026 Mark AB (markik)
// SPDX-License-Identifier: MIT OR Apache-2.0

#![cfg(feature = "graphshell-admission")]

use std::collections::VecDeque;

use cleromancy::moirai::clotho::EntropySource;
use cleromancy::servitor::{Cap, Grant, Mode, Subject};
use cleromancy::{
    AstrologyChart, AstrologyMoment, AstrologyPosition, CREATE_CONCURRENCE_INTENT, CleromancyApp,
    CleromancyHost, CleromancySessionAuthority, ReadingEngine, ReadingError, a0_fixture,
};
use graphshell::client::RetainedEndpointSession;
use graphshell::lifecycle::AdmittedEndpointContext;
use graphshell::native::endpoint_catalog::ResidentEndpointCatalog;
use graphshell_endpoint::{
    PresentationSource, ProjectionCatalog, ProjectionNoticeSource, ProjectionSource,
};
use graphshell_local::LocalCarrier;
use graphshell_protocol::{
    CapabilityProfile, IntentResult, PresentationCapability, ProjectionSession, ResourceRequest,
};
use muniment::MemoryBackend;

#[test]
fn resident_sessions_isolate_projection_state_and_share_reading_truth() {
    let (context, field) = a0_fixture();
    let chart = fixture_chart("2026-08-06T12:00:00Z", 180_000);
    let facts = chart.facts(1_000).unwrap();
    let reading = ReadingEngine::calculate(&context, &field).unwrap();
    let mut entropy = FixedEntropy::new([0x20, 0x21]);
    let mut host = CleromancyHost::empty(MemoryBackend::new());
    host.insert_astrology_chart(&chart, 1_000).unwrap();
    let reading_session = host
        .record_reading_session_at_with_entropy(
            &context,
            &field,
            &reading,
            1_786_017_600_000,
            Some("a20-resident-authority".to_string()),
            &mut entropy,
        )
        .unwrap();

    let writer = AdmittedEndpointContext::new(
        ProjectionSession("admitted:cleromancy-a20-writer".to_string()),
        [0x20; 32],
    );
    let reader = AdmittedEndpointContext::new(
        ProjectionSession("admitted:cleromancy-a20-reader".to_string()),
        [0x21; 32],
    );
    let mut app = CleromancyApp::new(host);
    app.servitors_mut()
        .grant(Grant::new(
            Subject::new(writer.subject()),
            Cap::scope("cleromancy/intents").unwrap(),
            Mode::Write,
        ))
        .unwrap();
    let authority = CleromancySessionAuthority::new(app);
    let mut catalog = ResidentEndpointCatalog::new();
    authority
        .register_catalog(&mut catalog, "cleromancy", "Local Cleromancy readings")
        .unwrap();

    let mut reader_endpoint = catalog.open("cleromancy", &reader).unwrap();
    let mut reader_descriptor = reader_endpoint.describe();
    let reader_request = reader_descriptor.projections.remove(0).request;
    assert_eq!(reader_request.session, reader.session().clone());
    let before = reader_endpoint.snapshot(reader_request.clone()).unwrap();
    let reader_resource = before
        .presentation
        .offers
        .values()
        .flatten()
        .next()
        .expect("the reader snapshot discloses a portable card")
        .resource;
    assert_eq!(
        reader_endpoint
            .resource(ResourceRequest {
                session: reader.session().clone(),
                resource: reader_resource,
            })
            .unwrap()
            .session,
        reader.session().clone()
    );
    assert!(
        reader_endpoint
            .resource(ResourceRequest {
                session: writer.session().clone(),
                resource: reader_resource,
            })
            .is_err()
    );

    let writer_endpoint = catalog.open("cleromancy", &writer).unwrap();
    let mut writer_session = RetainedEndpointSession::over(
        Box::new(LocalCarrier::new(writer_endpoint, |_, _| {
            Err("A20 requests a fresh snapshot after a write".to_string())
        })),
        CapabilityProfile::new([PresentationCapability::PortableCard]),
    )
    .unwrap();
    let mounted = writer_session.mount(0).unwrap();
    assert_eq!(mounted, writer.session().clone());
    let (target, action) = writer_session
        .client()
        .accessibility_tree(&mounted, writer_session.profile())
        .unwrap()
        .children
        .into_iter()
        .find_map(|item| {
            item.actions
                .into_iter()
                .find(|action| action.intent.0 == CREATE_CONCURRENCE_INTENT)
                .map(|action| (item.instance, action))
        })
        .expect("the admitted writer sees the concurrence action");
    let (mut draft, invocation) = writer_session
        .open_action_draft(&mounted, target, &action.intent.0)
        .unwrap();
    draft
        .choose("astrology_facts_digest", facts.digest())
        .unwrap();
    draft
        .choose("reading_session_id", &reading_session.id)
        .unwrap();
    assert_eq!(
        writer_session
            .submit_action_draft(&invocation, &mut draft)
            .unwrap(),
        IntentResult::Accepted
    );

    let notice = reader_endpoint
        .poll_notice()
        .unwrap()
        .expect("a graph mutation rings the other admitted session");
    assert_eq!(notice.session, reader.session().clone());
    assert_eq!(notice.epoch, before.scene.epoch);
    assert!(notice.revision > before.scene.revision);
    assert_eq!(reader_endpoint.poll_notice().unwrap(), None);

    let after = reader_endpoint.snapshot(reader_request).unwrap();
    assert_eq!(after.session, reader.session().clone());
    assert_eq!(after.scene.epoch, notice.epoch);
    assert_eq!(after.scene.revision, notice.revision);
    assert!(
        after
            .presentation
            .offers
            .values()
            .flatten()
            .any(|offer| offer.semantics.label == "Pattern occasion")
    );
}

fn fixture_chart(moment: &str, moon_longitude_millidegrees: u32) -> AstrologyChart {
    AstrologyChart::new(
        "fixture-positions/v1",
        "fixture-engine",
        "fixture-ephemeris",
        AstrologyMoment::global(moment),
        [
            AstrologyPosition::new("moon", moon_longitude_millidegrees, 1_000),
            AstrologyPosition::new("sun", 0, 0),
        ],
    )
    .unwrap()
}

struct FixedEntropy {
    words: VecDeque<u64>,
}

impl FixedEntropy {
    fn new(words: impl IntoIterator<Item = u64>) -> Self {
        Self {
            words: words.into_iter().collect(),
        }
    }
}

impl EntropySource for FixedEntropy {
    fn next_u64(&mut self) -> Result<u64, ReadingError> {
        self.words
            .pop_front()
            .ok_or_else(|| ReadingError::Entropy("fixture exhausted".to_string()))
    }
}
