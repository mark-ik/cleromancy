// Copyright 2026 Mark AB (markik)
// SPDX-License-Identifier: MIT OR Apache-2.0

mod support;

use cleromancy::moirai::clotho::EntropySource;
use cleromancy::{
    CleromancyApp, CleromancyHost, Reading, ReadingEngine, ReadingError, SelectionMode, a2_fixture,
};
use graphshell_protocol::Carrier;
use muniment::RedbBackend;

use support::{fixture_carrier, truth_bytes};

#[test]
fn sealed_external_evidence_qualifies_and_replays_after_endpoint_shutdown() {
    let (context, field) = a2_fixture();
    let temp = tempfile::tempdir().unwrap();
    let backend = RedbBackend::open(temp.path().join("cleromancy-a2.redb")).unwrap();
    let mut app = CleromancyApp::new(CleromancyHost::empty(backend.clone()));
    let before = truth_bytes(&app.host);
    let mut carrier = fixture_carrier();
    let external = app.mount_external(&mut carrier, 0).unwrap();
    let evidence = external.seal(&context).unwrap();
    let report = evidence.verify(&context).unwrap();

    assert_eq!(report.source_cards, 2);
    assert_eq!(evidence.evidence_digest.len(), 64);
    assert!(
        evidence
            .sources
            .iter()
            .any(|source| source.presentation == "Field notes")
    );
    carrier.shutdown().unwrap();
    assert_eq!(truth_bytes(&app.host), before);

    let baseline = ReadingEngine::calculate(&context, &field).unwrap();
    assert_eq!(baseline.candidate_id, "threshold");
    assert_eq!(baseline.receipt.qualified_weights, [6, 3, 2]);
    assert!(baseline.receipt.enrichment.is_none());

    let calculated = ReadingEngine::calculate_enriched(&context, &field, &evidence).unwrap();
    assert_eq!(calculated.schema, "cleromancy.reading/v2");
    assert_eq!(calculated.receipt.schema, "cleromancy.receipt/v2");
    assert_eq!(calculated.receipt.mode, SelectionMode::Calculated);
    assert_eq!(calculated.candidate_id, "measure");
    assert_eq!(calculated.receipt.qualified_weights, [6, 7, 2]);
    assert_eq!(calculated.receipt.total_weight, 15);
    let qualification = calculated.receipt.enrichment.as_ref().unwrap();
    assert_eq!(qualification.weight_additions, [0, 4, 0]);
    assert_eq!(
        qualification.candidate_terms,
        [
            Vec::<String>::new(),
            ["field", "harmony", "notes", "radio"]
                .map(str::to_string)
                .to_vec(),
            Vec::new(),
        ]
    );
    assert_eq!(
        ReadingEngine::replay(&context, &field, &calculated.receipt).unwrap(),
        calculated
    );

    let mut entropy = FixedEntropy::new([7, 0x1122, 0x3344]);
    let cast =
        ReadingEngine::cast_enriched_with(&context, &field, &evidence, &mut entropy).unwrap();
    assert_eq!(cast.receipt.mode, SelectionMode::Cast);
    assert_eq!(cast.receipt.sample, Some(7));
    assert_eq!(cast.candidate_id, "measure");
    assert_eq!(
        ReadingEngine::replay(&context, &field, &cast.receipt).unwrap(),
        cast
    );

    let mut tampered = calculated.receipt.clone();
    tampered.enrichment.as_mut().unwrap().evidence.sources[0]
        .title
        .push_str(" changed");
    assert!(matches!(
        ReadingEngine::replay(&context, &field, &tampered),
        Err(ReadingError::InvalidEnrichment(_))
    ));

    app.host.insert_reading(&context, &calculated).unwrap();
    let html = app.receipt_html().unwrap();
    assert!(html.contains("externally-qualified"));
    assert!(html.contains("Evidence digest"));
    assert!(html.contains("External additions"));
    assert!(html.contains(&evidence.evidence_digest));

    pollster::block_on(app.host.persist(456)).unwrap();
    drop(app);
    let reopened = pollster::block_on(CleromancyHost::open(backend)).unwrap();
    let reading_key = reopened
        .graph()
        .get_node_by_url(&format!("cleromancy://reading/{}", calculated.id))
        .unwrap()
        .0;
    let stored: Reading = serde_json::from_value(
        reopened
            .facet_value(reading_key, cleromancy::host::READING_FACET)
            .unwrap()
            .clone(),
    )
    .unwrap();
    assert_eq!(stored, calculated);
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
