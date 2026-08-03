// Copyright 2026 Mark AB (markik)
// SPDX-License-Identifier: MIT OR Apache-2.0

use cleromancy::servitor::{Cap, Grant, Mode, Subject};
use cleromancy::{
    CleromancyApp, CleromancyHost, READ_INTENT, READ_SCOPE, ROLL_INTENT, Reading, ReadingEngine,
    ReadingIntentPayload, RollIntentPayload, SELECT_INTENT, a0_fixture,
};
use graphshell_local::LocalCarrier;
use graphshell_protocol::{
    Carrier, CarrierRequestBody, CarrierResponseBody, IntentInvocation, IntentResult,
    ProjectionSnapshot,
};
use muniment::MemoryBackend;

#[test]
fn bound_authorized_consumer_reads_selects_and_rolls_through_the_wire() {
    let (context, field) = a0_fixture();
    let mut host = CleromancyHost::empty(MemoryBackend::new());
    host.insert_context(&context).unwrap();
    let mut app = CleromancyApp::new(host);
    let subject = Subject::new([7; 32]);
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
    let target = context_target(&first);
    let actions = first
        .presentation
        .offers_for(target)
        .unwrap()
        .iter()
        .flat_map(|offer| offer.semantics.actions.iter())
        .map(|action| action.intent.0.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(actions, [READ_INTENT, ROLL_INTENT, SELECT_INTENT].into());

    let read_intent = invocation(
        &first,
        target,
        READ_INTENT,
        &ReadingIntentPayload::read(field.clone()),
    );
    assert_eq!(
        request_intent(&mut carrier, read_intent.clone()),
        IntentResult::Accepted
    );
    let notice = carrier.take_notice().expect("read emits a revision notice");
    assert!(notice.revision > first.scene.revision);
    assert!(matches!(
        request_intent(&mut carrier, read_intent),
        IntentResult::Stale { .. }
    ));

    let second = snapshot(&mut carrier, &request);
    assert_eq!(
        request_intent(
            &mut carrier,
            invocation(
                &second,
                context_target(&second),
                SELECT_INTENT,
                &ReadingIntentPayload::select(field.clone()),
            ),
        ),
        IntentResult::Accepted
    );
    assert!(carrier.take_notice().is_some());

    let third = snapshot(&mut carrier, &request);
    let roll = RollIntentPayload::new(6).with_label("brass d6");
    assert_eq!(
        request_intent(
            &mut carrier,
            invocation(&third, context_target(&third), ROLL_INTENT, &roll),
        ),
        IntentResult::Accepted
    );
    assert!(carrier.take_notice().is_some());

    let readings = readings(carrier.endpoint());
    assert_eq!(readings.len(), 3);
    let calculated = readings
        .iter()
        .find(|reading| {
            reading.system == field.system
                && reading.receipt.mode == cleromancy::SelectionMode::Calculated
        })
        .unwrap();
    assert_eq!(
        ReadingEngine::replay(&context, &field, &calculated.receipt).unwrap(),
        calculated.clone()
    );
    let selected = readings
        .iter()
        .find(|reading| {
            reading.system == field.system
                && reading.receipt.mode == cleromancy::SelectionMode::Cast
        })
        .unwrap();
    assert_eq!(
        ReadingEngine::replay(&context, &field, &selected.receipt).unwrap(),
        selected.clone()
    );
    let rolled = readings
        .iter()
        .find(|reading| reading.system == "cleromancy.die/d6")
        .unwrap();
    assert_eq!(
        ReadingEngine::replay(&context, &roll.field(), &rolled.receipt).unwrap(),
        rolled.clone()
    );
    assert!((1..=6).contains(&rolled.candidate_id.parse::<u32>().unwrap()));
    assert_eq!(carrier.endpoint().servitors().audit().revision(), 4);
}

#[test]
fn payload_identity_cannot_replace_transport_binding_or_servitor_authority() {
    let (context, field) = a0_fixture();
    let mut host = CleromancyHost::empty(MemoryBackend::new());
    host.insert_context(&context).unwrap();
    let subject = Subject::new([9; 32]);
    let mut app = CleromancyApp::new(host);
    app.bind_intent_subject(subject);
    let mut carrier = LocalCarrier::new(app, |_, _| Err("resume is not used".to_string()));
    let request = discover_request(&mut carrier);
    let first = snapshot(&mut carrier, &request);
    let intent = invocation(
        &first,
        context_target(&first),
        READ_INTENT,
        &ReadingIntentPayload::read(field.clone()),
    );

    carrier.endpoint_mut().clear_intent_subject();
    assert!(matches!(
        request_intent(&mut carrier, intent.clone()),
        IntentResult::Rejected { reason } if reason.contains("transport")
    ));
    carrier.endpoint_mut().bind_intent_subject(subject);
    assert!(matches!(
        request_intent(&mut carrier, intent.clone()),
        IntentResult::Rejected { reason } if reason.contains("Servitor")
    ));
    assert_eq!(carrier.endpoint().host.graph().nodes().count(), 1);
    assert_eq!(carrier.endpoint().servitors().audit().revision(), 0);
    assert!(carrier.take_notice().is_none());

    carrier
        .endpoint_mut()
        .servitors_mut()
        .grant(Grant::new(
            subject,
            Cap::scope(READ_SCOPE).unwrap(),
            Mode::Write,
        ))
        .unwrap();
    assert!(matches!(
        request_intent(
            &mut carrier,
            invocation(
                &first,
                context_target(&first),
                SELECT_INTENT,
                &ReadingIntentPayload::select(field),
            ),
        ),
        IntentResult::Rejected { reason } if reason.contains("Servitor")
    ));
    assert_eq!(request_intent(&mut carrier, intent), IntentResult::Accepted);
    assert_eq!(carrier.endpoint().host.graph().nodes().count(), 2);
    assert_eq!(carrier.endpoint().servitors().audit().revision(), 2);
    assert!(carrier.take_notice().is_some());
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

fn context_target(snapshot: &ProjectionSnapshot) -> sceno::InstanceId {
    snapshot
        .presentation
        .bindings
        .iter()
        .find(|binding| {
            snapshot
                .presentation
                .offers_for(binding.instance)
                .is_some_and(|offers| {
                    offers
                        .iter()
                        .any(|offer| !offer.semantics.actions.is_empty())
                })
        })
        .expect("snapshot advertises commands on its context")
        .instance
}

fn invocation(
    snapshot: &ProjectionSnapshot,
    target: sceno::InstanceId,
    intent: &str,
    payload: &impl serde::Serialize,
) -> IntentInvocation {
    IntentInvocation {
        session: snapshot.session.clone(),
        target,
        observed_epoch: snapshot.scene.epoch,
        observed_revision: snapshot.scene.revision,
        intent: intent.to_string(),
        payload: serde_json::to_vec(payload).unwrap(),
    }
}

fn request_intent(carrier: &mut impl Carrier, intent: IntentInvocation) -> IntentResult {
    match carrier.request(CarrierRequestBody::Intent(intent)).unwrap() {
        CarrierResponseBody::Intent(result) => result,
        response => panic!("unexpected intent response: {response:?}"),
    }
}

fn readings(app: &CleromancyApp<MemoryBackend>) -> Vec<Reading> {
    app.host
        .graph()
        .nodes()
        .filter_map(|(key, _)| {
            app.host
                .facet_value(key, cleromancy::host::READING_FACET)
                .and_then(|value| serde_json::from_value(value.clone()).ok())
        })
        .collect()
}
