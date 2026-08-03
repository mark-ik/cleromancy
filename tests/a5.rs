// Copyright 2026 Mark AB (markik)
// SPDX-License-Identifier: MIT OR Apache-2.0

use cleromancy::{Candidate, CleromancyHost, ContextSnapshot, Field, HostError, ReadingEngine};
use mere::kernel::graph::{ProvenanceSubKind, RelationKind};
use muniment::MemoryBackend;

#[test]
fn graph_resident_field_replays_after_the_callers_field_is_gone() {
    let context = ContextSnapshot::new("A game turn", "example.game-context/v1")
        .with_fact("move", "cross the river")
        .with_tags(["risk", "water"]);
    let field = Field::new(
        "example.game-prompt/v1",
        "contextual-weight/v1",
        [
            Candidate::new("wait", "Wait", "Let the board change first.").with_tags(["risk"]),
            Candidate::new("cross", "Cross", "Commit to the exposed route.").with_tags(["water"]),
        ],
    );
    let field_digest = field.digest();
    let reading = ReadingEngine::calculate(&context, &field).unwrap();
    let mut host = CleromancyHost::empty(MemoryBackend::new());
    host.insert_reading(&context, &field, &reading).unwrap();

    assert!(
        host.graph()
            .get_node_by_url(&format!("cleromancy://field/{field_digest}"))
            .is_some()
    );
    assert_eq!(
        host.graph()
            .relations()
            .filter(|relation| {
                relation.kind == RelationKind::Provenance(ProvenanceSubKind::GeneratedFrom)
            })
            .count(),
        2
    );

    drop(context);
    drop(field);
    assert_eq!(host.replay_reading(&reading).unwrap(), reading);

    let mut unbound = reading.clone();
    unbound.receipt.field_digest = "0".repeat(64);
    assert!(matches!(
        host.replay_reading(&unbound),
        Err(HostError::MissingReadingDependency { kind: "field", .. })
    ));
}
