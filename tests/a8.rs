// Copyright 2026 Mark AB (markik)
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::collections::VecDeque;

use cleromancy::moirai::clotho::EntropySource;
use cleromancy::{
    CleromancyHost, ReadingError, TarotPack, TarotQualification, ThreeCardPosition,
    ThreeCardRelationKind,
};
use mere::kernel::graph::{RelationKind, SemanticSubKind};
use muniment::MemoryBackend;

#[test]
fn fixed_three_card_spread_is_authored_replayable_and_graph_visible() {
    let context = cleromancy::ContextSnapshot::new("A turning point", "cleromancy.a8-context/v1")
        .with_fact("question", "What deserves attention next?");
    let field = TarotPack::rws_major_arcana().field(TarotQualification::Uniform);
    let mut entropy = FixedEntropy::new([0, 0x11, 0x22, 1, 0x33, 0x44, 2, 0x55, 0x66, 0x77, 0x88]);
    let mut host = CleromancyHost::empty(MemoryBackend::new());

    let (session, spread, readings) = host
        .record_three_card_spread_at_with_entropy(
            &context,
            &field,
            1_753_000_000_000,
            Some("a8-proof".to_string()),
            &mut entropy,
        )
        .unwrap();

    assert_eq!(readings.len(), 3);
    assert_eq!(
        session
            .placements
            .iter()
            .map(|placement| placement.position.as_str())
            .collect::<Vec<_>>(),
        ["foundation", "tension", "next_step"]
    );
    assert_eq!(
        spread
            .placements
            .iter()
            .map(|placement| placement.position)
            .collect::<Vec<_>>(),
        ThreeCardPosition::ALL
    );
    assert_eq!(spread.relations[0].kind, ThreeCardRelationKind::Questions);
    assert_eq!(spread.relations[1].kind, ThreeCardRelationKind::NextStep);
    assert_eq!(host.replay_three_card_spread(&spread).unwrap(), readings);

    let semantic = host
        .graph()
        .relations()
        .filter(|relation| {
            relation.kind == RelationKind::Semantic(SemanticSubKind::Questions)
                || relation.kind == RelationKind::Semantic(SemanticSubKind::NextStep)
        })
        .count();
    assert_eq!(semantic, 2);
    assert!(host.graph().relations().any(|relation| matches!(
        relation.kind,
        RelationKind::Containment(_) | RelationKind::Provenance(_)
    )));

    let mut tampered = spread.clone();
    tampered.relations[0].label = "a different story".to_string();
    assert!(tampered.validate().is_err());
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
