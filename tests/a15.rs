use std::collections::VecDeque;

use cleromancy::moirai::clotho::EntropySource;
use cleromancy::{
    AstrologyChart, AstrologyMoment, AstrologyPosition, CleromancyApp, CleromancyHost, Concurrence,
    ConcurrenceError, ConcurrenceMember, ContextSnapshot, HostError, ReadingEngine, ReadingError,
    TarotPack, TarotQualification,
};
use graphshell_client::ResolvedContent;
use mere::kernel::graph::{ContainmentSubKind, RelationKind};
use muniment::MemoryBackend;

#[test]
fn concurrence_is_canonical_and_rejects_ambiguous_members() {
    let first = format!("cleromancy://first/{}", "1".repeat(64));
    let second = format!("cleromancy://second/{}", "2".repeat(64));
    let concurrence = Concurrence::new(
        1_000,
        "Two systems",
        [
            ConcurrenceMember::new("second", &second),
            ConcurrenceMember::new("first", &first),
        ],
    )
    .unwrap();
    assert_eq!(concurrence.members[0].address, first);
    assert_eq!(concurrence.members[1].address, second);

    assert_eq!(
        Concurrence::new(
            1_000,
            "Duplicate",
            [
                ConcurrenceMember::new("first", &first),
                ConcurrenceMember::new("second", &first),
            ],
        ),
        Err(ConcurrenceError::Invalid("duplicate member".to_string()))
    );
    let mut tampered = concurrence;
    tampered.id = "0".repeat(64);
    assert_eq!(
        tampered.validate(),
        Err(ConcurrenceError::Invalid("identity".to_string()))
    );
}

#[test]
fn astrology_and_tarot_share_an_inspectable_occasion_without_a_causal_claim() {
    let chart = AstrologyChart::new(
        "fixture-positions/v1",
        "fixture-engine",
        "fixture-ephemeris",
        AstrologyMoment::global("2026-08-05T12:00:00Z"),
        [
            AstrologyPosition::new("moon", 180_000, 1_000),
            AstrologyPosition::new("sun", 0, 0),
        ],
    )
    .unwrap();
    let facts = chart.facts(1_000).unwrap();
    let context = ContextSnapshot::new("A cross-system reading", "cleromancy.a15-context/v1")
        .with_fact("question", "What deserves attention now?")
        .with_tags(["reflection"]);
    let field = TarotPack::rws_major_arcana().field(TarotQualification::Uniform);
    let mut entropy = FixedEntropy::new([16, 0x11, 0x22, 0x33, 0x44]);
    let reading = ReadingEngine::cast_with(&context, &field, &mut entropy).unwrap();
    let backend = MemoryBackend::new();
    let mut host = CleromancyHost::empty(backend.clone());
    let (_chart_key, facts_key) = host.insert_astrology_chart(&chart, 1_000).unwrap();
    let session = host
        .record_reading_session_at_with_entropy(
            &context,
            &field,
            &reading,
            1_785_931_200_000,
            Some("a15-pattern".to_string()),
            &mut entropy,
        )
        .unwrap();
    let session_key = host
        .graph()
        .get_node_by_url(&format!("cleromancy://session/{}", session.id))
        .unwrap()
        .0;
    let concurrence =
        Concurrence::astrology_reading(session.created_at_ms, &facts.digest(), &session.id)
            .unwrap();
    let concurrence_key = host.insert_concurrence(&concurrence).unwrap();

    assert_eq!(
        host.graph()
            .relations()
            .filter(|relation| {
                relation.from == concurrence_key
                    && relation.kind
                        == RelationKind::Containment(ContainmentSubKind::CollectionMember)
            })
            .count(),
        2
    );
    assert!(!host.graph().relations().any(|relation| {
        (relation.from == facts_key && relation.to == session_key)
            || (relation.from == session_key && relation.to == facts_key)
    }));
    assert_eq!(host.replay_concurrence(&concurrence).unwrap(), concurrence);

    pollster::block_on(host.persist(15)).unwrap();
    let host = pollster::block_on(CleromancyHost::open(backend)).unwrap();
    assert_eq!(host.replay_concurrence(&concurrence).unwrap(), concurrence);
    let before = host.graph().nodes().count();
    let missing = Concurrence::new(
        session.created_at_ms,
        "Missing member",
        [
            ConcurrenceMember::new(
                "reading-session",
                format!("cleromancy://session/{}", session.id),
            ),
            ConcurrenceMember::new("unknown", "cleromancy://unknown/member"),
        ],
    )
    .unwrap();
    let mut host = host;
    assert!(matches!(
        host.insert_concurrence(&missing),
        Err(HostError::MissingConcurrenceMember { .. })
    ));
    assert_eq!(host.graph().nodes().count(), before);

    let mut app = CleromancyApp::new(host);
    let cards = app.mount_local().unwrap();
    assert!(cards.iter().any(|presentation| {
        matches!(&presentation.content, ResolvedContent::PortableCard(card)
        if card.title == "Pattern occasion"
            && card.values.iter().any(|value| {
                value.label == "Claim" && value.value.contains("no causal")
            }))
    }));
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
