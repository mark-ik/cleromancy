// Copyright 2026 Mark AB (markik)
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::collections::VecDeque;

use cleromancy::moirai::clotho::EntropySource;
use cleromancy::servitor::{Cap, Grant, Mode, Subject};
use cleromancy::{
    a0_fixture, AstrologyChart, AstrologyMoment, AstrologyPosition, CleromancyApp, CleromancyHost,
    ReadingEngine, ReadingError, CREATE_CONCURRENCE_INTENT,
};
use graphshell::client::RetainedEndpointSession;
use graphshell_local::LocalCarrier;
use graphshell_protocol::{CapabilityProfile, IntentResult, PresentationCapability};
use muniment::MemoryBackend;

#[test]
fn retained_graphshell_session_submits_cleromancy_concurrence_from_exact_choices() {
    let (context, field) = a0_fixture();
    let facts = fixture_chart("2026-08-06T12:00:00Z", 180_000)
        .facts(1_000)
        .unwrap();
    let reading = ReadingEngine::calculate(&context, &field).unwrap();
    let mut entropy = FixedEntropy::new([0x17, 0x18]);
    let mut host = CleromancyHost::empty(MemoryBackend::new());
    host.insert_astrology_chart(&fixture_chart("2026-08-06T12:00:00Z", 180_000), 1_000)
        .unwrap();
    let reading_session = host
        .record_reading_session_at_with_entropy(
            &context,
            &field,
            &reading,
            1_786_017_600_000,
            Some("a17-retained-session".to_string()),
            &mut entropy,
        )
        .unwrap();
    let subject = Subject::new([0x17; 32]);
    let mut app = CleromancyApp::new(host);
    app.bind_intent_subject(subject);
    app.servitors_mut()
        .grant(Grant::new(
            subject,
            Cap::scope("cleromancy/intents").unwrap(),
            Mode::Write,
        ))
        .unwrap();

    let mut retained = RetainedEndpointSession::over(
        Box::new(LocalCarrier::new(app, |_, _| {
            Err("A17 requests a fresh snapshot after a write".to_string())
        })),
        CapabilityProfile::new([PresentationCapability::PortableCard]),
    )
    .unwrap();
    let session = retained.mount(0).unwrap();
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
        .expect("saved facts card advertises the bounded concurrence action");

    let (mut draft, invocation_target) = retained
        .open_action_draft(&session, target, &action.intent.0)
        .unwrap();
    let revision_before = retained
        .client()
        .acknowledgement(&session)
        .unwrap()
        .revision
        .0;
    let semantics = draft.semantics();
    assert_eq!(semantics.label, "Save pattern occasion");
    assert_eq!(semantics.fields.len(), 2);
    assert_eq!(semantics.fields[0].name, "astrology_facts_digest");
    assert_eq!(
        semantics.fields[0]
            .choices
            .iter()
            .map(|choice| choice.value.as_str())
            .collect::<Vec<_>>(),
        vec![facts.digest()]
    );
    assert_eq!(semantics.fields[1].name, "reading_session_id");
    assert_eq!(
        semantics.fields[1]
            .choices
            .iter()
            .map(|choice| choice.value.as_str())
            .collect::<Vec<_>>(),
        vec![reading_session.id.as_str()]
    );
    assert!(retained
        .submit_action_draft(&invocation_target, &mut draft)
        .unwrap_err()
        .contains("requires field astrology_facts_digest"));

    draft
        .choose("astrology_facts_digest", facts.digest())
        .unwrap();
    draft
        .choose("reading_session_id", &reading_session.id)
        .unwrap();
    assert_eq!(
        retained
            .submit_action_draft(&invocation_target, &mut draft)
            .unwrap(),
        IntentResult::Accepted
    );
    retained.resnapshot(&session).unwrap();
    assert!(
        retained
            .client()
            .acknowledgement(&session)
            .unwrap()
            .revision
            .0
            > revision_before,
        "resnapshot must observe the accepted endpoint write"
    );
    assert!(retained
        .resolve_all(&session)
        .unwrap()
        .into_iter()
        .any(|(_, presentation)| presentation.semantics.label == "Pattern occasion"));
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
