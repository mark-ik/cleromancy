use cleromancy::{Candidate, ComposerError, FIELD_COMPOSER_SCHEMA, Field, FieldComposer};

#[test]
fn field_composer_emits_the_exact_field_and_round_trips() {
    let first = Candidate::new("threshold", "Attend to the threshold", "Notice the change.")
        .with_tags(["change", "reflection"])
        .with_base_weight(2);
    let second = Candidate::new("measure", "Measure the structure", "Name the constraint.");
    let expected = Field::new(
        "cleromancy.composed/v1",
        "contextual-weight/v1",
        [first.clone(), second.clone()],
    );
    let mut composer = FieldComposer::new("cleromancy.composed/v1", "contextual-weight/v1");
    composer.add_candidate(first).unwrap();
    composer.add_candidate(second).unwrap();
    assert_eq!(composer.schema, FIELD_COMPOSER_SCHEMA);
    assert_eq!(composer.clone().finish().unwrap(), expected);

    let encoded = serde_json::to_vec(&composer).unwrap();
    let decoded: FieldComposer = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(decoded.finish().unwrap(), expected);
}

#[test]
fn field_composer_rejects_structural_drafts_before_dispatch() {
    let candidate = Candidate::new("same", "Same", "Same.");
    let mut duplicate = FieldComposer::new("system", "rules");
    duplicate.add_candidate(candidate.clone()).unwrap();
    assert_eq!(
        duplicate.add_candidate(candidate),
        Err(ComposerError::InvalidCandidate("same".to_string()))
    );

    let mut zero_weight = FieldComposer::new("system", "rules");
    assert_eq!(
        zero_weight.add_candidate(Candidate::new("zero", "Zero", "Zero.").with_base_weight(0)),
        Err(ComposerError::EmptyWeight("zero".to_string()))
    );

    assert_eq!(
        FieldComposer::new("", "rules")
            .with_candidate(Candidate::new("one", "One", "One."))
            .unwrap()
            .finish(),
        Err(ComposerError::EmptySystem)
    );
    assert_eq!(
        FieldComposer::new("system", "rules").finish(),
        Err(ComposerError::EmptyField)
    );
}
