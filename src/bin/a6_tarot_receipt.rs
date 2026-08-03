// Copyright 2026 Mark AB (markik)
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::collections::VecDeque;
use std::path::{Path, PathBuf};

use cleromancy::moirai::clotho::EntropySource;
use cleromancy::{
    CleromancyApp, CleromancyHost, ContextSnapshot, Reading, ReadingEngine, ReadingError,
    TarotPack, TarotQualification,
};
use mere::kernel::graph::{ProvenanceSubKind, RelationKind};
use muniment::MemoryBackend;
use serde::Serialize;

#[derive(Serialize)]
struct TarotReceipt {
    schema: &'static str,
    pack: TarotPack,
    pack_digest: String,
    context_digest: String,
    uniform_field_digest: String,
    contextual_field_digest: String,
    uniform_proof_entropy: &'static str,
    uniform: Reading,
    contextual: Reading,
    graph_nodes: usize,
    provenance_relations: usize,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments = std::env::args_os().skip(1);
    let html_path = arguments
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("receipts/a6-tarot.html"));
    let json_path = arguments
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("receipts/a6-tarot.json"));

    let context = ContextSnapshot::new("A turning point", "cleromancy.tarot-context/v1")
        .with_fact("question", "What kind of change is already underway?")
        .with_tags(["change", "cycle"]);
    let pack = TarotPack::rws_major_arcana();
    let uniform_field = pack.field(TarotQualification::Uniform);
    let contextual_field = pack.field(TarotQualification::Contextual);
    let mut entropy = FixedEntropy::new([16, 0x11, 0x22]);
    let uniform = ReadingEngine::cast_with(&context, &uniform_field, &mut entropy)?;
    let contextual = ReadingEngine::calculate(&context, &contextual_field)?;

    if uniform.candidate_id != "major-16-tower" || uniform.receipt.qualified_weights != vec![1; 22]
    {
        return Err("uniform Tarot proof did not remain uniform".into());
    }
    if contextual.candidate_id != "major-10-wheel-of-fortune"
        || contextual.receipt.qualified_weights[10] != 3
    {
        return Err("contextual Tarot proof did not disclose its qualification".into());
    }

    let mut host = CleromancyHost::empty(MemoryBackend::new());
    host.insert_reading(&context, &uniform_field, &uniform)?;
    host.insert_reading(&context, &contextual_field, &contextual)?;
    if host.replay_reading(&uniform)? != uniform || host.replay_reading(&contextual)? != contextual
    {
        return Err("graph-resident Tarot replay changed a reading".into());
    }
    let graph_nodes = host.graph().nodes().count();
    let provenance_relations = host
        .graph()
        .relations()
        .filter(|relation| {
            relation.kind == RelationKind::Provenance(ProvenanceSubKind::GeneratedFrom)
        })
        .count();
    let receipt = TarotReceipt {
        schema: "cleromancy.proof/a6-tarot-pack-v1",
        pack_digest: pack.digest(),
        context_digest: context.digest(),
        uniform_field_digest: uniform_field.digest(),
        contextual_field_digest: contextual_field.digest(),
        uniform_proof_entropy: "fixed fixture words; production cast uses operating-system entropy",
        pack,
        uniform,
        contextual,
        graph_nodes,
        provenance_relations,
    };
    let mut app = CleromancyApp::new(host);
    write(&html_path, app.receipt_html()?.as_bytes())?;
    write(&json_path, &serde_json::to_vec_pretty(&receipt)?)?;
    println!(
        "sealed {} cards into two fields; uniform and contextual replay passed; wrote {} and {}",
        receipt.pack.candidates.len(),
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
