use cleromancy::{
    ASTROLOGY_FACTS_ALGORITHM, ASTROLOGY_FACTS_SCHEMA, AspectKind, AstrologyChart, AstrologyError,
    AstrologyMoment, AstrologyPosition, ZodiacSign,
};

fn chart() -> AstrologyChart {
    AstrologyChart::new(
        "ephemeris-adapter/example/v1",
        "example-engine",
        "example-ephemeris",
        AstrologyMoment::at("2026-08-04T16:00:00Z", 40_712_800, -74_006_000),
        [
            AstrologyPosition::new("moon", 180_000, 1_000),
            AstrologyPosition::new("sun", 0, 0).with_retrograde(false),
            AstrologyPosition::new("venus", 60_000, -500),
        ],
    )
    .unwrap()
}

#[test]
fn chart_derives_replayable_structured_facts() {
    let chart = chart();
    let encoded = serde_json::to_vec(&chart).unwrap();
    let decoded: AstrologyChart = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(decoded, chart);
    assert_eq!(chart.positions[0].body, "moon");
    assert_eq!(chart.positions[1].body, "sun");
    assert_eq!(chart.positions[2].body, "venus");

    let facts = chart.facts(1_000).unwrap();
    assert_eq!(facts.schema, ASTROLOGY_FACTS_SCHEMA);
    assert_eq!(facts.algorithm, ASTROLOGY_FACTS_ALGORITHM);
    assert_eq!(facts.placements[0].sign, ZodiacSign::Libra);
    assert_eq!(facts.placements[0].degree_millidegrees, 0);
    assert_eq!(facts.placements[1].sign, ZodiacSign::Aries);
    assert_eq!(facts.placements[2].sign, ZodiacSign::Gemini);
    assert_eq!(facts.aspects.len(), 3);
    assert_eq!(facts.aspects[0].kind, AspectKind::Opposition);
    assert_eq!(facts.aspects[1].kind, AspectKind::Trine);
    assert_eq!(facts.aspects[2].kind, AspectKind::Sextile);
    facts.verify(&chart).unwrap();
}

#[test]
fn chart_metadata_and_facts_bind_the_calculation_boundary() {
    let chart = chart();
    let mut facts = chart.facts(1_000).unwrap();
    facts.aspects[0].orb_millidegrees = 1;
    assert_eq!(
        facts.verify(&chart),
        Err(AstrologyError::FactsMismatch("aspects"))
    );

    let mut altered = chart.clone();
    altered.engine = "different-engine".to_string();
    assert_ne!(altered.digest(), chart.digest());
    assert_eq!(
        chart.facts(180_001),
        Err(AstrologyError::InvalidOrb(180_001))
    );
}

#[test]
fn chart_rejects_ambiguous_or_invalid_positions() {
    assert_eq!(
        AstrologyChart::new(
            "algorithm",
            "engine",
            "ephemeris",
            AstrologyMoment::global(""),
            [AstrologyPosition::new("sun", 0, 0)],
        ),
        Err(AstrologyError::Empty("instant_utc"))
    );
    assert_eq!(
        AstrologyChart::new(
            "algorithm",
            "engine",
            "ephemeris",
            AstrologyMoment::global("2026-08-04T16:00:00Z"),
            [
                AstrologyPosition::new("sun", 0, 0),
                AstrologyPosition::new("sun", 1, 0),
            ],
        ),
        Err(AstrologyError::InvalidBody("sun".to_string()))
    );
    assert_eq!(
        AstrologyChart::new(
            "algorithm",
            "engine",
            "ephemeris",
            AstrologyMoment::at("2026-08-04T16:00:00Z", 0, 0),
            [AstrologyPosition::new("sun", 360_000, 0)],
        ),
        Err(AstrologyError::InvalidLongitude(360_000))
    );
}
