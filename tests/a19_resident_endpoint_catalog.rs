// Copyright 2026 Mark AB (markik)
// SPDX-License-Identifier: MIT OR Apache-2.0

#![cfg(feature = "graphshell-admission")]

use std::collections::VecDeque;

use cleromancy::moirai::clotho::EntropySource;
use cleromancy::servitor::{Cap, Grant, Mode, Subject};
use cleromancy::{
    AstrologyChart, AstrologyMoment, AstrologyPosition, CREATE_CONCURRENCE_INTENT, CleromancyApp,
    CleromancyHost, ReadingEngine, ReadingError, a0_fixture,
};
use graphshell::client::RetainedEndpointSession;
use graphshell::lifecycle::{AdmittedEndpointContext, BindAdmittedSession};
use graphshell::native::endpoint_catalog::ResidentEndpointCatalog;
use graphshell_local::LocalCarrier;
use graphshell_protocol::{
    CapabilityProfile, IntentResult, PresentationCapability, ProjectionSession,
};
use muniment::MemoryBackend;

#[test]
fn resident_catalog_opens_cleromancy_under_the_admitted_session() {
    let (context, field) = a0_fixture();
    let chart = fixture_chart("2026-08-06T12:00:00Z", 180_000);
    let facts = chart.facts(1_000).unwrap();
    let reading = ReadingEngine::calculate(&context, &field).unwrap();
    let mut entropy = FixedEntropy::new([0x19, 0x1A]);
    let mut host = CleromancyHost::empty(MemoryBackend::new());
    host.insert_astrology_chart(&chart, 1_000).unwrap();
    let reading_session = host
        .record_reading_session_at_with_entropy(
            &context,
            &field,
            &reading,
            1_786_017_600_000,
            Some("a19-resident-catalog".to_string()),
            &mut entropy,
        )
        .unwrap();

    let admitted = AdmittedEndpointContext::new(
        ProjectionSession("admitted:cleromancy-a19".to_string()),
        [0x19; 32],
    );
    let subject = Subject::new(admitted.subject());
    let mut app = CleromancyApp::new(host);
    app.servitors_mut()
        .grant(Grant::new(
            subject,
            Cap::scope("cleromancy/intents").unwrap(),
            Mode::Write,
        ))
        .unwrap();
    let mut app = Some(app);
    let mut catalog = ResidentEndpointCatalog::new();
    catalog
        .register_notifying("cleromancy", "Local Cleromancy readings", move |context| {
            app.take()
                .map(|app| app.bind_admitted_session(context))
                .ok_or_else(|| "the one-session fixture is already open".to_string())
        })
        .unwrap();
    let endpoint = catalog.open("cleromancy", &admitted).unwrap();

    let mut retained = RetainedEndpointSession::over(
        Box::new(LocalCarrier::new(endpoint, |_, _| {
            Err("A19 requests a fresh snapshot after a write".to_string())
        })),
        CapabilityProfile::new([PresentationCapability::PortableCard]),
    )
    .unwrap();
    let session = retained.mount(0).unwrap();
    assert_eq!(session, admitted.session().clone());
    let (target, action) = retained
        .client()
        .accessibility_tree(&session, retained.profile())
        .unwrap()
        .children
        .into_iter()
        .find_map(|item| {
            item.actions
                .into_iter()
                .find(|action| action.intent.0 == CREATE_CONCURRENCE_INTENT)
                .map(|action| (item.instance, action))
        })
        .expect("the catalog-bound subject sees the concurrence action");
    let (mut draft, invocation) = retained
        .open_action_draft(&session, target, &action.intent.0)
        .unwrap();
    draft
        .choose("astrology_facts_digest", facts.digest())
        .unwrap();
    draft
        .choose("reading_session_id", &reading_session.id)
        .unwrap();
    assert_eq!(
        retained
            .submit_action_draft(&invocation, &mut draft)
            .unwrap(),
        IntentResult::Accepted
    );
    retained.resnapshot(&session).unwrap();
    assert!(
        retained
            .resolve_all(&session)
            .unwrap()
            .into_iter()
            .any(|(_, card)| card.semantics.label == "Pattern occasion")
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
