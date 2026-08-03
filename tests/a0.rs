// Copyright 2026 Mark AB (markik)
// SPDX-License-Identifier: MIT OR Apache-2.0

use cleromancy::moirai::clotho::EntropySource;
use cleromancy::servitor::{Cap, Grant, Mode, Subject};
use cleromancy::{
    CleromancyApp, CleromancyHost, ReadingEngine, ReadingError, SelectionMode, a0_fixture,
};
use graphshell_client::ResolvedContent;
use mere::kernel::graph::{ProvenanceSubKind, RelationKind};
use muniment::{Backend, RedbBackend};

#[test]
fn calculated_reading_is_byte_stable_and_replays() {
    let (context, field) = a0_fixture();
    let first = ReadingEngine::calculate(&context, &field).unwrap();
    let second = ReadingEngine::calculate(&context, &field).unwrap();

    assert_eq!(first, second);
    assert_eq!(first.receipt.mode, SelectionMode::Calculated);
    assert_eq!(first.receipt.qualified_weights, vec![6, 3, 2]);
    assert_eq!(first.receipt.total_weight, 11);
    assert_eq!(
        serde_json::to_vec(&first).unwrap(),
        serde_json::to_vec(&second).unwrap()
    );
    assert_eq!(
        ReadingEngine::replay(&context, &field, &first.receipt).unwrap(),
        first
    );
}

#[test]
fn cast_discloses_a_bounded_sample_and_replays_without_entropy() {
    let (context, field) = a0_fixture();
    let mut entropy = FixedEntropy::new([u64::MAX, 7, 0x1122, 0x3344]);
    let reading = ReadingEngine::cast_with(&context, &field, &mut entropy).unwrap();

    assert_eq!(reading.receipt.mode, SelectionMode::Cast);
    assert_eq!(reading.receipt.sample, Some(7));
    assert_eq!(reading.candidate_id, "measure");
    assert_eq!(
        reading.receipt.event_nonce.as_deref(),
        Some("00000000000011220000000000003344")
    );
    assert_eq!(
        ReadingEngine::replay(&context, &field, &reading.receipt).unwrap(),
        reading
    );
}

#[test]
fn mere_truth_reopens_and_graphshell_resolves_every_card() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("cleromancy.redb");
    let backend = RedbBackend::open(&path).unwrap();
    let (context, field) = a0_fixture();
    let calculated = ReadingEngine::calculate(&context, &field).unwrap();
    let mut entropy = FixedEntropy::new([u64::MAX, 7, 0x1122, 0x3344]);
    let cast = ReadingEngine::cast_with(&context, &field, &mut entropy).unwrap();

    {
        let mut host = CleromancyHost::empty(backend.clone());
        host.insert_reading(&context, &calculated).unwrap();
        host.insert_reading(&context, &cast).unwrap();
        pollster::block_on(host.persist(123)).unwrap();
    }

    let before = pollster::block_on(backend.get(cleromancy::host::HOST_SLOT))
        .unwrap()
        .unwrap();
    let mut host = pollster::block_on(CleromancyHost::open(backend.clone())).unwrap();
    assert!(host.was_reopened());
    assert_eq!(host.graph().nodes().count(), 3);
    assert_eq!(
        host.graph()
            .relations()
            .filter(|relation| {
                relation.kind == RelationKind::Provenance(ProvenanceSubKind::GeneratedFrom)
            })
            .count(),
        2
    );
    pollster::block_on(host.persist(123)).unwrap();
    let after = pollster::block_on(backend.get(cleromancy::host::HOST_SLOT))
        .unwrap()
        .unwrap();
    assert_eq!(before, after, "reopen and re-save changed stored truth");

    let mut app = CleromancyApp::new(host);
    let presentations = app.mount_local().unwrap();
    assert_eq!(presentations.len(), 3);
    assert!(
        presentations
            .iter()
            .all(|presentation| matches!(presentation.content, ResolvedContent::PortableCard(_)))
    );
    let html = app.receipt_html().unwrap();
    assert!(html.contains("Qualified readings, with their workings"));
    assert!(html.contains("calculated"));
    assert!(html.contains("cast"));
    assert!(html.matches("<line ").count() >= 2);
}

#[test]
fn servitor_access_keeps_scope_and_mode_boundaries() {
    let backend = muniment::MemoryBackend::new();
    let host = CleromancyHost::empty(backend);
    let mut app = CleromancyApp::new(host);
    let subject = Subject::new([7; 32]);
    let readings = Cap::scope("cleromancy/readings").unwrap();
    app.servitors_mut()
        .grant(Grant::new(subject, readings, Mode::Read))
        .unwrap();

    assert!(app.servitors().allows(
        subject,
        &Cap::scope("cleromancy/readings/one").unwrap(),
        Mode::Read
    ));
    assert!(!app.servitors().allows(
        subject,
        &Cap::scope("cleromancy/readings/one").unwrap(),
        Mode::Write
    ));
    assert!(!app.servitors().allows(
        subject,
        &Cap::scope("cleromancy/context").unwrap(),
        Mode::Read
    ));
    let _real_gate = app.servitors().gate();
}

struct FixedEntropy {
    words: std::collections::VecDeque<u64>,
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
            .ok_or_else(|| ReadingError::Entropy("fixed source exhausted".to_string()))
    }
}
