// Copyright 2026 Mark AB (markik)
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::collections::VecDeque;
use std::path::{Path, PathBuf};

use cleromancy::moirai::clotho::EntropySource;
use cleromancy::{
    CleromancyApp, CleromancyHost, Reading, ReadingEngine, ReadingError, ReadingSession,
    Reflection, a0_fixture,
};
use mere::kernel::graph::RelationKind;
use muniment::MemoryBackend;
use serde::Serialize;

#[derive(Serialize)]
struct SessionReceipt {
    schema: &'static str,
    context_digest: String,
    field_digest: String,
    reading: Reading,
    sessions: Vec<ReadingSession>,
    reflection: Reflection,
    event_entropy: &'static str,
    graph_nodes: usize,
    graph_relations: usize,
    semantic_relations: usize,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments = std::env::args_os().skip(1);
    let html_path = arguments
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("receipts/a7-session.html"));
    let json_path = arguments
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("receipts/a7-session.json"));

    let (context, field) = a0_fixture();
    let reading = ReadingEngine::calculate(&context, &field)?;
    let mut entropy = FixedEntropy::new([0x11, 0x22, 0x33, 0x44, 0x55, 0x66]);
    let mut host = CleromancyHost::empty(MemoryBackend::new());
    let first = host.record_reading_session_at_with_entropy(
        &context,
        &field,
        &reading,
        1_735_689_600_000,
        Some("a7-first".to_string()),
        &mut entropy,
    )?;
    let second = host.record_reading_session_at_with_entropy(
        &context,
        &field,
        &reading,
        1_735_689_601_000,
        Some("a7-second".to_string()),
        &mut entropy,
    )?;
    if first.id == second.id || host.replay_session(&first)? != vec![reading.clone()] {
        return Err("separate reading occasions did not retain their shared result".into());
    }
    let reflection = host.record_reflection_at_with_entropy(
        &first,
        1_735_689_602_000,
        "The repeated result changed the question, not the result.",
        &mut entropy,
    )?;
    let receipt = SessionReceipt {
        schema: "cleromancy.proof/a7-reading-session-v1",
        context_digest: context.digest(),
        field_digest: field.digest(),
        reading,
        sessions: vec![first, second],
        reflection,
        event_entropy: "fixed fixture words; production sessions use operating-system entropy",
        graph_nodes: host.graph().nodes().count(),
        graph_relations: host.graph().relations().count(),
        semantic_relations: host
            .graph()
            .relations()
            .filter(|relation| matches!(relation.kind, RelationKind::Semantic(_)))
            .count(),
    };
    let mut app = CleromancyApp::new(host);
    write(&html_path, app.receipt_html()?.as_bytes())?;
    write(&json_path, &serde_json::to_vec_pretty(&receipt)?)?;
    println!(
        "saved two reading sessions and one reflection; wrote {} and {}",
        html_path.display(),
        json_path.display()
    );
    Ok(())
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

fn write(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent().filter(|path| !path.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, bytes)
}
