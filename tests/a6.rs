// Copyright 2026 Mark AB (markik)
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::collections::{BTreeSet, VecDeque};

use cleromancy::moirai::clotho::EntropySource;
use cleromancy::{
    Candidate, CleromancyApp, CleromancyHost, ContextSnapshot, EXTERNAL_TERM_WEIGHT_RULE, Field,
    ReadingEngine, ReadingError, TarotPack, TarotQualification, UNIFORM_RULE,
};
use mere::kernel::graph::{ProvenanceSubKind, RelationKind};
use muniment::MemoryBackend;

#[test]
fn major_arcana_pack_has_stable_complete_ordering() {
    let pack = TarotPack::rws_major_arcana();
    assert_eq!(pack.candidates.len(), 22);
    assert_eq!(
        pack.candidates
            .iter()
            .map(|candidate| candidate.id.as_str())
            .collect::<BTreeSet<_>>()
            .len(),
        22
    );
    assert_eq!(pack.candidates[0].id, "major-00-fool");
    assert_eq!(pack.candidates[8].id, "major-08-strength");
    assert_eq!(pack.candidates[11].id, "major-11-justice");
    assert_eq!(pack.candidates[21].id, "major-21-world");
    assert_eq!(pack.digest().len(), 64);
}

#[test]
fn uniform_and_contextual_tarot_disclose_different_qualification() {
    let context = ContextSnapshot::new("A turning point", "cleromancy.tarot-context/v1")
        .with_fact("question", "What kind of change is already underway?")
        .with_tags(["change", "cycle"]);
    let pack = TarotPack::rws_major_arcana();
    let uniform_field = pack.field(TarotQualification::Uniform);
    let contextual_field = pack.field(TarotQualification::Contextual);
    assert_ne!(uniform_field.digest(), contextual_field.digest());

    let mut entropy = FixedEntropy::new([16, 0x11, 0x22]);
    let uniform = ReadingEngine::cast_with(&context, &uniform_field, &mut entropy).unwrap();
    assert_eq!(uniform.candidate_id, "major-16-tower");
    assert_eq!(uniform.receipt.qualified_weights, vec![1; 22]);
    assert_eq!(uniform.receipt.total_weight, 22);
    assert_eq!(
        uniform.receipt.algorithm,
        "os-csprng/rejection-u64+uniform/v1"
    );

    let contextual = ReadingEngine::calculate(&context, &contextual_field).unwrap();
    assert_eq!(contextual.candidate_id, "major-10-wheel-of-fortune");
    assert_eq!(contextual.receipt.qualified_weights[10], 3);
    assert_eq!(contextual.receipt.total_weight, 24);
    assert_eq!(contextual.receipt.algorithm, "contextual-weight/max/v1");

    let mut host = CleromancyHost::empty(MemoryBackend::new());
    host.insert_reading(&context, &uniform_field, &uniform)
        .unwrap();
    host.insert_reading(&context, &contextual_field, &contextual)
        .unwrap();
    assert_eq!(host.graph().nodes().count(), 5);
    assert_eq!(
        host.graph()
            .relations()
            .filter(|relation| {
                relation.kind == RelationKind::Provenance(ProvenanceSubKind::GeneratedFrom)
            })
            .count(),
        4
    );
    assert_eq!(host.replay_reading(&uniform).unwrap(), uniform);
    assert_eq!(host.replay_reading(&contextual).unwrap(), contextual);

    let first_html = CleromancyApp::new(host).receipt_html().unwrap();
    let mut second_host = CleromancyHost::empty(MemoryBackend::new());
    second_host
        .insert_reading(&context, &uniform_field, &uniform)
        .unwrap();
    second_host
        .insert_reading(&context, &contextual_field, &contextual)
        .unwrap();
    let second_html = CleromancyApp::new(second_host).receipt_html().unwrap();
    assert_eq!(first_html, second_html);
}

#[test]
fn rule_names_are_enforced_instead_of_treated_as_labels() {
    let context = ContextSnapshot::new("Rule audit", "cleromancy.rule-audit/v1");
    let unknown = Field::new(
        "example.unknown/v1",
        "invented-rule/v1",
        [Candidate::new("one", "One", "One")],
    );
    assert!(matches!(
        ReadingEngine::calculate(&context, &unknown),
        Err(ReadingError::UnsupportedRule(rule)) if rule == "invented-rule/v1"
    ));

    let not_uniform = Field::new(
        "example.not-uniform/v1",
        UNIFORM_RULE,
        [Candidate::new("heavy", "Heavy", "Heavy").with_base_weight(2)],
    );
    assert!(matches!(
        ReadingEngine::calculate(&context, &not_uniform),
        Err(ReadingError::NonUniformCandidate { candidate, weight: 2 })
            if candidate == "heavy"
    ));

    let uniform = TarotPack::rws_major_arcana().field(TarotQualification::Uniform);
    assert!(matches!(
        ReadingEngine::calculate(&context, &uniform),
        Err(ReadingError::QualificationRequiresCast(rule)) if rule == UNIFORM_RULE
    ));

    let external = Field::new(
        "example.external/v1",
        EXTERNAL_TERM_WEIGHT_RULE,
        [Candidate::new("one", "One", "One")],
    );
    assert!(matches!(
        ReadingEngine::calculate(&context, &external),
        Err(ReadingError::QualificationEvidenceRequired(rule))
            if rule == EXTERNAL_TERM_WEIGHT_RULE
    ));
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
