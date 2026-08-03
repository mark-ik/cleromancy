// Copyright 2026 Mark AB (markik)
// SPDX-License-Identifier: MIT OR Apache-2.0

mod support;

use cleromancy::{CleromancyApp, CleromancyHost, ReadingEngine, a0_fixture, a1_fixture};
use graphshell_protocol::Carrier;
use muniment::MemoryBackend;

use support::{fixture_carrier, truth_bytes};

#[test]
fn external_projection_mounts_beside_local_truth_and_correlation_is_a_receipt() {
    let (local_context, local_field) = a0_fixture();
    let local_reading = ReadingEngine::calculate(&local_context, &local_field).unwrap();
    let mut local_host = CleromancyHost::empty(MemoryBackend::new());
    local_host
        .insert_reading(&local_context, &local_reading)
        .unwrap();
    let before = truth_bytes(&local_host);
    let local_session = local_host.session();
    let mut app = CleromancyApp::new(local_host);
    assert_eq!(app.mount_local().unwrap().len(), 2);

    let mut carrier = fixture_carrier();
    let external = app.mount_external(&mut carrier, 0).unwrap();
    assert_ne!(external.session, local_session);
    assert!(app.client().mounted(&local_session).is_some());
    assert!(app.client().mounted(&external.session).is_some());
    assert_eq!(external.presentations.len(), 2);
    assert_eq!(truth_bytes(&app.host), before);

    let (context, _) = a1_fixture();
    let report = external.correlate(&context).unwrap();
    assert_eq!(report.source_cards, 2);
    assert!(report.matches.iter().any(|matched| {
        matched.presentation == "Field notes"
            && matched.terms == ["field", "harmony", "notes", "radio"]
    }));
    assert_eq!(
        serde_json::to_vec(&report).unwrap(),
        serde_json::to_vec(&external.correlate(&context).unwrap()).unwrap()
    );
    assert_eq!(truth_bytes(&app.host), before);

    let html = app.enrichment_receipt_html(&external, &report).unwrap();
    assert!(html.contains("Fixture source remains source-owned"));
    assert!(html.contains("cleromancy.enrichment/lexical-overlap/v1"));
    assert!(html.contains("Field notes"));
    carrier.shutdown().unwrap();
}
