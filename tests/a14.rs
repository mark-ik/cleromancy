use cleromancy::{
    AstrologyAdapter, AstrologyChart, AstrologyError, AstrologyMoment, AstrologyPosition,
    CleromancyApp, CleromancyHost, HostError, calculate_with_adapter,
};
use graphshell_client::ResolvedContent;
use mere::kernel::graph::{ProvenanceSubKind, RelationKind};
use muniment::MemoryBackend;

struct FixtureAdapter;

impl AstrologyAdapter for FixtureAdapter {
    type Error = String;

    fn calculate(&self, moment: &AstrologyMoment) -> Result<AstrologyChart, Self::Error> {
        AstrologyChart::new(
            "fixture-ephemeris/v1",
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

#[test]
fn adapter_receipt_requires_the_requested_moment() {
    let moment = AstrologyMoment::global("2026-08-04T16:00:00Z");
    let chart = calculate_with_adapter(&FixtureAdapter, &moment).unwrap();
    assert_eq!(chart.moment, moment);

    struct WrongMomentAdapter;
    impl AstrologyAdapter for WrongMomentAdapter {
        type Error = String;

        fn calculate(&self, _moment: &AstrologyMoment) -> Result<AstrologyChart, Self::Error> {
            AstrologyChart::new(
                "fixture-ephemeris/v1",
                "fixture-engine",
                "fixture-ephemeris",
                AstrologyMoment::global("2026-08-05T16:00:00Z"),
                [AstrologyPosition::new("sun", 0, 0)],
            )
            .map_err(|error| error.to_string())
        }
    }

    assert_eq!(
        calculate_with_adapter(&WrongMomentAdapter, &moment),
        Err(AstrologyError::FactsMismatch("adapter moment"))
    );
}

#[test]
fn astrology_chart_and_facts_are_graph_truth_and_project_as_cards() {
    let moment = AstrologyMoment::at("2026-08-04T16:00:00Z", 40_712_800, -74_006_000);
    let chart = calculate_with_adapter(&FixtureAdapter, &moment).unwrap();
    let facts = chart.facts(1_000).unwrap();
    let chart_digest = chart.digest();
    let facts_digest = facts.digest();
    let backend = MemoryBackend::new();
    let mut host = CleromancyHost::empty(backend.clone());
    let (_chart_key, _facts_key) = host.insert_astrology_chart(&chart, 1_000).unwrap();
    pollster::block_on(host.persist(7)).unwrap();
    let host = pollster::block_on(CleromancyHost::open(backend)).unwrap();

    assert!(
        host.graph()
            .get_node_by_url(&format!("cleromancy://astrology/chart/{chart_digest}"))
            .is_some()
    );
    assert!(
        host.graph()
            .get_node_by_url(&format!("cleromancy://astrology/facts/{facts_digest}"))
            .is_some()
    );
    assert_eq!(
        host.graph()
            .relations()
            .filter(|relation| {
                relation.kind == RelationKind::Provenance(ProvenanceSubKind::GeneratedFrom)
            })
            .count(),
        1
    );
    assert_eq!(host.replay_astrology_facts(&facts).unwrap(), facts);
    assert_eq!(
        host.astrology_chart_for_digest(&chart_digest).unwrap(),
        chart
    );
    assert_eq!(
        host.astrology_facts_for_digest(&facts_digest).unwrap(),
        facts
    );

    let mut missing = facts.clone();
    missing.chart_digest = "0".repeat(64);
    assert!(matches!(
        host.replay_astrology_facts(&missing),
        Err(HostError::MissingReadingDependency {
            kind: "astrology chart",
            ..
        })
    ));

    let mut app = CleromancyApp::new(host);
    let cards = app.mount_local().unwrap();
    assert!(cards.iter().any(|presentation| {
        matches!(
            &presentation.content,
            ResolvedContent::PortableCard(card) if card.title == "Astrology chart (fixture-engine)"
        )
    }));
    assert!(cards.iter().any(|presentation| {
        matches!(
            &presentation.content,
            ResolvedContent::PortableCard(card) if card.title == "Astrology facts (2026-08-04T16:00:00Z)"
        )
    }));
}
