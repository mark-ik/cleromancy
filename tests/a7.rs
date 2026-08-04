// Copyright 2026 Mark AB (markik)
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::collections::VecDeque;

use cleromancy::moirai::clotho::EntropySource;
use cleromancy::{
    CleromancyHost, Reading, ReadingEngine, ReadingError, ReadingSession, Reflection, a0_fixture,
};
use muniment::MemoryBackend;

#[test]
fn repeated_calculated_reads_are_separate_sessions_with_immutable_reflection() {
    let (context, field) = a0_fixture();
    let reading = ReadingEngine::calculate(&context, &field).unwrap();
    let mut host = CleromancyHost::empty(MemoryBackend::new());
    let mut entropy = FixedEntropy::new([0x11, 0x22, 0x33, 0x44, 0x55, 0x66]);

    let first = host
        .record_reading_session_at_with_entropy(
            &context,
            &field,
            &reading,
            1_735_689_600_000,
            Some("first-read".to_string()),
            &mut entropy,
        )
        .unwrap();
    let second = host
        .record_reading_session_at_with_entropy(
            &context,
            &field,
            &reading,
            1_735_689_601_000,
            Some("second-read".to_string()),
            &mut entropy,
        )
        .unwrap();
    assert_ne!(first.id, second.id);
    assert_eq!(first.placements[0].position, "focus");
    assert_eq!(first.placements[0].reading_id, reading.id);
    assert_eq!(host.replay_session(&first).unwrap(), vec![reading.clone()]);
    assert_eq!(host.replay_session(&second).unwrap(), vec![reading.clone()]);

    let reflection = host
        .record_reflection_at_with_entropy(
            &first,
            1_735_689_602_000,
            "The repeated result changed the question, not the result.",
            &mut entropy,
        )
        .unwrap();
    assert_eq!(reflection.session_id, first.id);
    assert_eq!(host.graph().nodes().count(), 6);
    assert_eq!(sessions(&host).len(), 2);
    assert_eq!(reflections(&host), vec![reflection]);
    assert_eq!(readings(&host), vec![reading]);
}

#[test]
fn session_and_reflection_identity_reject_tampering() {
    let (context, field) = a0_fixture();
    let reading = ReadingEngine::calculate(&context, &field).unwrap();
    let mut session = ReadingSession::single(
        7,
        "00000000000000010000000000000002",
        context.digest(),
        field.digest(),
        reading.id,
        None,
    )
    .unwrap();
    session.placements[0].position = "past".to_string();
    assert!(session.validate().is_err());

    let mut reflection = Reflection::new(
        session.id,
        8,
        "00000000000000030000000000000004",
        "A first note.",
    )
    .unwrap();
    reflection.body = "A changed note.".to_string();
    assert!(reflection.validate().is_err());
}

fn readings(host: &CleromancyHost<MemoryBackend>) -> Vec<Reading> {
    domain_values(host, cleromancy::host::READING_FACET)
}

fn sessions(host: &CleromancyHost<MemoryBackend>) -> Vec<ReadingSession> {
    domain_values(host, cleromancy::host::SESSION_FACET)
}

fn reflections(host: &CleromancyHost<MemoryBackend>) -> Vec<Reflection> {
    domain_values(host, cleromancy::host::REFLECTION_FACET)
}

fn domain_values<T: serde::de::DeserializeOwned>(
    host: &CleromancyHost<MemoryBackend>,
    facet: &str,
) -> Vec<T> {
    host.graph()
        .nodes()
        .filter_map(|(key, _)| {
            host.facet_value(key, facet)
                .and_then(|value| serde_json::from_value(value.clone()).ok())
        })
        .collect()
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
