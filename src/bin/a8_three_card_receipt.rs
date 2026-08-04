// Copyright 2026 Mark AB (markik)
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::collections::VecDeque;
use std::path::{Path, PathBuf};

use cleromancy::moirai::clotho::EntropySource;
use cleromancy::{
    CleromancyApp, CleromancyHost, ContextSnapshot, Reading, ReadingError, TarotPack,
    TarotQualification, ThreeCardSpread,
};
use mere::kernel::graph::{RelationKind, SemanticSubKind};
use muniment::MemoryBackend;
use serde::Serialize;

#[derive(Serialize)]
struct SpreadReceipt {
    schema: &'static str,
    context_digest: String,
    field_digest: String,
    session_id: String,
    spread: ThreeCardSpread,
    readings: Vec<Reading>,
    replayed: bool,
    entropy: &'static str,
    graph_nodes: usize,
    graph_relations: usize,
    authored_relationships: usize,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments = std::env::args_os().skip(1);
    let html_path = arguments
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("receipts/a8-three-card.html"));
    let json_path = arguments
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("receipts/a8-three-card.json"));

    let context = ContextSnapshot::new("A turning point", "cleromancy.a8-receipt-context/v1")
        .with_fact("question", "What deserves attention next?")
        .with_tags(["change", "reflection"]);
    let field = TarotPack::rws_major_arcana().field(TarotQualification::Uniform);
    let mut entropy = FixedEntropy::new([0, 0x11, 0x22, 1, 0x33, 0x44, 2, 0x55, 0x66, 0x77, 0x88]);
    let mut host = CleromancyHost::empty(MemoryBackend::new());
    let (session, spread, readings) = host.record_three_card_spread_at_with_entropy(
        &context,
        &field,
        1_753_000_000_000,
        Some("a8-proof".to_string()),
        &mut entropy,
    )?;
    let replayed = host.replay_three_card_spread(&spread)? == readings;
    if !replayed {
        return Err("three-card spread replay changed a reading".into());
    }
    let authored_relationships = host
        .graph()
        .relations()
        .filter(|relation| {
            matches!(
                relation.kind,
                RelationKind::Semantic(SemanticSubKind::Questions)
                    | RelationKind::Semantic(SemanticSubKind::NextStep)
            )
        })
        .count();
    if authored_relationships != 2 {
        return Err("three-card authored relationship graph proof failed".into());
    }
    let receipt = SpreadReceipt {
        schema: "cleromancy.proof/a8-three-card-spread-v1",
        context_digest: context.digest(),
        field_digest: field.digest(),
        session_id: session.id,
        spread,
        readings,
        replayed,
        entropy: "fixed fixture words; production casts use operating-system entropy",
        graph_nodes: host.graph().nodes().count(),
        graph_relations: host.graph().relations().count(),
        authored_relationships,
    };
    let mut app = CleromancyApp::new(host);
    write(&html_path, app.receipt_html()?.as_bytes())?;
    write(&json_path, &serde_json::to_vec_pretty(&receipt)?)?;
    println!(
        "saved an authored three-card spread and replay proof; wrote {} and {}",
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
