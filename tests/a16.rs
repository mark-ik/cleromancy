// Copyright 2026 Mark AB (markik)
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::collections::{BTreeMap, VecDeque};

use cleromancy::moirai::clotho::EntropySource;
use cleromancy::servitor::{Cap, Grant, Mode, Subject};
use cleromancy::{
    AstrologyChart, AstrologyMoment, AstrologyPosition, AstrologyReadingConcurrenceIntentPayload,
    CREATE_CONCURRENCE_INTENT, CleromancyApp, CleromancyHost, Concurrence, ReadingEngine,
    ReadingError, a0_fixture,
};
use graphshell_local::LocalCarrier;
use graphshell_protocol::{
    ActionFormError, AdvertisedAction, Carrier, CarrierRequestBody, CarrierResponseBody,
    IntentInvocation, IntentResult, PortableCardV1, ProjectionSnapshot, ResourceRequest,
};
use muniment::MemoryBackend;

#[test]
fn saved_facts_and_sessions_select_each_other_through_the_bound_pattern_action() {
    let (context, field) = a0_fixture();
    let first_chart = fixture_chart("2026-08-05T12:00:00Z", 180_000);
    let first_facts = first_chart.facts(1_000).unwrap();
    let second_chart = fixture_chart("2026-08-06T12:00:00Z", 90_000);
    let second_facts = second_chart.facts(1_000).unwrap();
    let reading = ReadingEngine::calculate(&context, &field).unwrap();
    let mut entropy = FixedEntropy::new([0x11, 0x22]);
    let mut host = CleromancyHost::empty(MemoryBackend::new());
    host.insert_astrology_chart(&first_chart, 1_000).unwrap();
    host.insert_astrology_chart(&second_chart, 1_000).unwrap();
    let session = host
        .record_reading_session_at_with_entropy(
            &context,
            &field,
            &reading,
            1_785_931_200_000,
            Some("a16-pattern".to_string()),
            &mut entropy,
        )
        .unwrap();
    let subject = Subject::new([0x16; 32]);
    let mut app = CleromancyApp::new(host);
    app.bind_intent_subject(subject);
    app.servitors_mut()
        .grant(Grant::new(
            subject,
            Cap::scope("cleromancy/intents").unwrap(),
            Mode::Write,
        ))
        .unwrap();
    let mut carrier = LocalCarrier::new(app, |_, _| Err("resume is not used".to_string()));

    let request = discover_request(&mut carrier);
    let first = snapshot(&mut carrier, &request);
    let facts_target = target_for_address(
        &mut carrier,
        &first,
        &format!("cleromancy://astrology/facts/{}", first_facts.digest()),
    );
    let session_target = target_for_address(
        &mut carrier,
        &first,
        &format!("cleromancy://session/{}", session.id),
    );
    let context_target = target_for_address(
        &mut carrier,
        &first,
        &format!("cleromancy://context/{}", context.digest()),
    );
    assert!(advertises(&first, facts_target, CREATE_CONCURRENCE_INTENT));
    assert!(advertises(
        &first,
        session_target,
        CREATE_CONCURRENCE_INTENT
    ));
    assert!(!advertises(
        &first,
        context_target,
        CREATE_CONCURRENCE_INTENT
    ));
    let facts_action = action_for(&first, facts_target, CREATE_CONCURRENCE_INTENT);
    let session_action = action_for(&first, session_target, CREATE_CONCURRENCE_INTENT);
    let mut expected_facts = vec![first_facts.digest(), second_facts.digest()];
    expected_facts.sort();
    let expected_sessions = vec![session.id.clone()];
    assert_concurrence_form(&facts_action, &expected_facts, &expected_sessions);
    assert_eq!(facts_action, session_action);
    assert_eq!(
        facts_action.compose_payload(&BTreeMap::from([
            (
                "astrology_facts_digest".to_string(),
                "not-advertised".to_string()
            ),
            ("reading_session_id".to_string(), session.id.clone()),
        ])),
        Err(ActionFormError::InvalidChoice {
            field: "astrology_facts_digest".to_string(),
            value: "not-advertised".to_string(),
        })
    );

    let node_count = carrier.endpoint().host.graph().nodes().count();
    let mismatched_payload = facts_action
        .compose_payload(&BTreeMap::from([
            ("astrology_facts_digest".to_string(), second_facts.digest()),
            ("reading_session_id".to_string(), session.id.clone()),
        ]))
        .unwrap();
    assert!(matches!(
        request_intent(
            &mut carrier,
            invocation(&first, facts_target, mismatched_payload),
        ),
        IntentResult::Rejected { reason } if reason.contains("not one selected member")
    ));
    assert!(carrier.take_notice().is_none());
    assert_eq!(carrier.endpoint().host.graph().nodes().count(), node_count);

    let selected_payload = session_action
        .compose_payload(&BTreeMap::from([
            ("astrology_facts_digest".to_string(), first_facts.digest()),
            ("reading_session_id".to_string(), session.id.clone()),
        ]))
        .unwrap();
    assert_eq!(
        serde_json::from_slice::<AstrologyReadingConcurrenceIntentPayload>(&selected_payload)
            .unwrap(),
        AstrologyReadingConcurrenceIntentPayload::new(first_facts.digest(), &session.id)
    );
    assert_eq!(
        request_intent(
            &mut carrier,
            invocation(&first, session_target, selected_payload),
        ),
        IntentResult::Accepted
    );
    assert!(carrier.take_notice().is_some());

    let second = snapshot(&mut carrier, &request);
    let concurrences =
        domain_values::<Concurrence>(carrier.endpoint(), cleromancy::CONCURRENCE_FACET);
    assert_eq!(concurrences.len(), 1);
    let concurrence = &concurrences[0];
    assert_eq!(
        carrier
            .endpoint()
            .host
            .replay_concurrence(concurrence)
            .unwrap(),
        *concurrence
    );
    assert!(concurrence.members.iter().any(|member| {
        member.address == format!("cleromancy://astrology/facts/{}", first_facts.digest())
    }));
    assert!(
        concurrence
            .members
            .iter()
            .any(|member| member.address == format!("cleromancy://session/{}", session.id))
    );
    assert!(cards(&mut carrier, &second).iter().any(|card| {
        card.title == "Pattern occasion"
            && card
                .values
                .iter()
                .any(|value| value.label == "Claim" && value.value.contains("no causal"))
    }));
}

fn fixture_chart(moment: &str, moon_longitude_millidegrees: u32) -> AstrologyChart {
    AstrologyChart::new(
        "fixture-positions/v1",
        "fixture-engine",
        "fixture-ephemeris",
        AstrologyMoment::global(moment),
        [
            AstrologyPosition::new("moon", moon_longitude_millidegrees, 1_000),
            AstrologyPosition::new("sun", 0, 0),
        ],
    )
    .unwrap()
}

fn discover_request(carrier: &mut impl Carrier) -> graphshell_protocol::ProjectionRequest {
    match carrier.request(CarrierRequestBody::Discover).unwrap() {
        CarrierResponseBody::Descriptor(descriptor) => descriptor.projections[0].request.clone(),
        response => panic!("unexpected discovery response: {response:?}"),
    }
}

fn snapshot(
    carrier: &mut impl Carrier,
    request: &graphshell_protocol::ProjectionRequest,
) -> ProjectionSnapshot {
    match carrier
        .request(CarrierRequestBody::Snapshot(request.clone()))
        .unwrap()
    {
        CarrierResponseBody::Snapshot(snapshot) => *snapshot,
        response => panic!("unexpected snapshot response: {response:?}"),
    }
}

fn target_for_address(
    carrier: &mut impl Carrier,
    snapshot: &ProjectionSnapshot,
    address: &str,
) -> sceno::InstanceId {
    snapshot
        .presentation
        .bindings
        .iter()
        .find(|binding| {
            card_for(carrier, snapshot, binding.instance)
                .values
                .iter()
                .any(|value| value.label == "Address" && value.value == address)
        })
        .expect("saved graph node has a portable card")
        .instance
}

fn advertises(snapshot: &ProjectionSnapshot, target: sceno::InstanceId, intent: &str) -> bool {
    snapshot
        .presentation
        .offers_for(target)
        .into_iter()
        .flatten()
        .flat_map(|offer| &offer.semantics.actions)
        .any(|action| action.intent.0 == intent)
}

fn action_for(
    snapshot: &ProjectionSnapshot,
    target: sceno::InstanceId,
    intent: &str,
) -> AdvertisedAction {
    snapshot
        .presentation
        .offers_for(target)
        .into_iter()
        .flatten()
        .flat_map(|offer| &offer.semantics.actions)
        .find(|action| action.intent.0 == intent)
        .cloned()
        .expect("target advertises the requested action")
}

fn assert_concurrence_form(action: &AdvertisedAction, facts: &[String], sessions: &[String]) {
    let form = action.input_form.as_ref().expect("bounded action form");
    assert_eq!(form.schema, cleromancy::CREATE_CONCURRENCE_SCHEMA);
    assert_eq!(form.fields.len(), 2);
    assert_eq!(form.fields[0].name, "astrology_facts_digest");
    assert_eq!(form.fields[0].label, "Astrology facts");
    assert_eq!(
        form.fields[0]
            .choices
            .iter()
            .map(|choice| choice.value.clone())
            .collect::<Vec<_>>(),
        facts
    );
    assert_eq!(form.fields[1].name, "reading_session_id");
    assert_eq!(form.fields[1].label, "Reading session");
    assert_eq!(
        form.fields[1]
            .choices
            .iter()
            .map(|choice| choice.value.clone())
            .collect::<Vec<_>>(),
        sessions
    );
}

fn invocation(
    snapshot: &ProjectionSnapshot,
    target: sceno::InstanceId,
    payload: Vec<u8>,
) -> IntentInvocation {
    IntentInvocation {
        session: snapshot.session.clone(),
        target,
        observed_epoch: snapshot.scene.epoch,
        observed_revision: snapshot.scene.revision,
        intent: CREATE_CONCURRENCE_INTENT.to_string(),
        payload,
    }
}

fn request_intent(carrier: &mut impl Carrier, intent: IntentInvocation) -> IntentResult {
    match carrier.request(CarrierRequestBody::Intent(intent)).unwrap() {
        CarrierResponseBody::Intent(result) => result,
        response => panic!("unexpected intent response: {response:?}"),
    }
}

fn card_for(
    carrier: &mut impl Carrier,
    snapshot: &ProjectionSnapshot,
    target: sceno::InstanceId,
) -> PortableCardV1 {
    let offer = snapshot
        .presentation
        .offers_for(target)
        .and_then(|offers| offers.first())
        .expect("instance has a card offer");
    let response = carrier
        .request(CarrierRequestBody::Resource(ResourceRequest {
            session: snapshot.session.clone(),
            resource: offer.resource,
        }))
        .unwrap();
    let CarrierResponseBody::Resource(response) = response else {
        panic!("expected a portable-card resource");
    };
    serde_json::from_slice(&response.bytes).unwrap()
}

fn cards(carrier: &mut impl Carrier, snapshot: &ProjectionSnapshot) -> Vec<PortableCardV1> {
    snapshot
        .presentation
        .bindings
        .iter()
        .map(|binding| card_for(carrier, snapshot, binding.instance))
        .collect()
}

fn domain_values<T: serde::de::DeserializeOwned>(
    app: &CleromancyApp<MemoryBackend>,
    facet: &str,
) -> Vec<T> {
    app.host
        .graph()
        .nodes()
        .filter_map(|(key, _)| {
            app.host
                .facet_value(key, facet)
                .and_then(|value| serde_json::from_value(value.clone()).ok())
        })
        .collect()
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
