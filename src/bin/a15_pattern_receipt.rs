// Copyright 2026 Mark AB (markik)
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::collections::VecDeque;
use std::path::{Path, PathBuf};

use cleromancy::moirai::clotho::EntropySource;
use cleromancy::{
    AstrologyAdapter, AstrologyChart, AstrologyFacts, AstrologyMoment, AstrologyPosition,
    CleromancyApp, CleromancyHost, Concurrence, ContextSnapshot, Reading, ReadingEngine,
    ReadingError, ReadingSession, TarotPack, TarotQualification, calculate_with_adapter,
};
use muniment::MemoryBackend;
use serde::Serialize;

#[derive(Serialize)]
struct PatternReceipt {
    schema: &'static str,
    calculation_source: &'static str,
    astrology_chart: AstrologyChart,
    astrology_facts: AstrologyFacts,
    tarot_reading: Reading,
    reading_session: ReadingSession,
    concurrence: Concurrence,
    concurrence_claim: &'static str,
    graph_nodes: usize,
    graph_relations: usize,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments = std::env::args_os().skip(1);
    let html_path = arguments
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("receipts/a15-pattern.html"));
    let json_path = arguments
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("receipts/a15-pattern.json"));

    let moment = AstrologyMoment::global("2026-08-05T12:00:00Z");
    let chart = calculate_with_adapter(&FixtureAdapter, &moment)?;
    let facts = chart.facts(1_000)?;
    let context = ContextSnapshot::new("A cross-system reading", "cleromancy.a15-context/v1")
        .with_fact("question", "What deserves attention now?")
        .with_tags(["reflection"]);
    let field = TarotPack::rws_major_arcana().field(TarotQualification::Uniform);
    let mut entropy = FixedEntropy::new([16, 0x11, 0x22, 0x33, 0x44]);
    let reading = ReadingEngine::cast_with(&context, &field, &mut entropy)?;
    let mut host = CleromancyHost::empty(MemoryBackend::new());
    host.insert_astrology_chart(&chart, 1_000)?;
    let session = host.record_reading_session_at_with_entropy(
        &context,
        &field,
        &reading,
        1_785_931_200_000,
        Some("a15-pattern".to_string()),
        &mut entropy,
    )?;
    let concurrence =
        Concurrence::astrology_reading(session.created_at_ms, &facts.digest(), &session.id)?;
    host.insert_concurrence(&concurrence)?;
    if host.replay_astrology_facts(&facts)? != facts
        || host.replay_session(&session)? != vec![reading.clone()]
        || host.replay_concurrence(&concurrence)? != concurrence
    {
        return Err("cross-system occasion did not replay".into());
    }

    let receipt = PatternReceipt {
        schema: "cleromancy.proof/a15-pattern-occasion-v1",
        calculation_source: "fixed fixture positions; not a production ephemeris",
        astrology_chart: chart,
        astrology_facts: facts,
        tarot_reading: reading,
        reading_session: session,
        concurrence,
        concurrence_claim: "consulted together; astrology did not qualify or cause the tarot cast",
        graph_nodes: host.graph().nodes().count(),
        graph_relations: host.graph().relations().count(),
    };
    let mut app = CleromancyApp::new(host);
    write(&html_path, app.receipt_html()?.as_bytes())?;
    write(&json_path, &serde_json::to_vec_pretty(&receipt)?)?;
    println!(
        "saved one cross-system pattern occasion; wrote {} and {}",
        html_path.display(),
        json_path.display()
    );
    Ok(())
}

struct FixtureAdapter;

impl AstrologyAdapter for FixtureAdapter {
    type Error = String;

    fn calculate(&self, moment: &AstrologyMoment) -> Result<AstrologyChart, Self::Error> {
        AstrologyChart::new(
            "fixture-positions/v1",
            "fixture-engine",
            "fixture-ephemeris",
            moment.clone(),
            [
                AstrologyPosition::new("moon", 180_000, 1_000),
                AstrologyPosition::new("sun", 0, 0),
            ],
        )
        .map_err(|error| error.to_string())
    }
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
