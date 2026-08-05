// Copyright 2026 Mark AB (markik)
// SPDX-License-Identifier: MIT OR Apache-2.0

use cleromancy::servitor::{Cap, Grant, Mode, Subject};
use cleromancy::{
    CleromancyApp, CleromancyHost, ReadingSession, THREE_CARD_SPREAD_INTENT, ThreeCardSpread,
    ThreeCardSpreadIntentPayload, a0_fixture,
};
use graphshell_local::LocalCarrier;
use graphshell_protocol::{
    Carrier, CarrierRequestBody, CarrierResponseBody, IntentInvocation, IntentResult,
    PortableCardV1, ProjectionSnapshot, ResourceRequest,
};
use muniment::MemoryBackend;

#[test]
fn bound_host_can_cast_the_authored_spread_through_the_wire() {
    let (context, field) = a0_fixture();
    let mut host = CleromancyHost::empty(MemoryBackend::new());
    host.insert_context(&context).unwrap();
    let subject = Subject::new([0xA9; 32]);
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
    let target = context_target(&first);
    let payload = ThreeCardSpreadIntentPayload::new(field).with_client_token("a9-spread");
    assert_eq!(
        request_intent(
            &mut carrier,
            invocation(&first, target, THREE_CARD_SPREAD_INTENT, &payload),
        ),
        IntentResult::Accepted
    );
    assert!(carrier.take_notice().is_some());

    let second = snapshot(&mut carrier, &request);
    let cards = cards(&mut carrier, &second);
    assert!(cards.iter().any(|card| card.title == "Three-card spread"));
    assert!(cards.iter().any(|card| {
        card.title == "Reading session"
            && card
                .values
                .iter()
                .any(|value| value.label == "Client token" && value.value == "a9-spread")
    }));
    let sessions =
        domain_values::<ReadingSession>(carrier.endpoint(), cleromancy::host::SESSION_FACET);
    let session = sessions
        .iter()
        .find(|session| session.client_token.as_deref() == Some("a9-spread"))
        .unwrap();
    assert_eq!(session.placements.len(), 3);
    let spreads = domain_values::<ThreeCardSpread>(
        carrier.endpoint(),
        cleromancy::host::THREE_CARD_SPREAD_FACET,
    );
    let spread = spreads.first().expect("spread facet after accepted intent");
    let readings = carrier
        .endpoint()
        .host
        .replay_three_card_spread(spread)
        .unwrap();
    assert_eq!(readings.len(), 3);
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
                .unwrap()
                .iter()
                .any(|offer| {
                    offer
                        .semantics
                        .actions
                        .iter()
                        .any(|action| action.intent.0 == THREE_CARD_SPREAD_INTENT)
                })
        })
        .unwrap()
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

fn cards(carrier: &mut impl Carrier, snapshot: &ProjectionSnapshot) -> Vec<PortableCardV1> {
    snapshot
        .presentation
        .bindings
        .iter()
        .flat_map(|binding| {
            snapshot
                .presentation
                .offers_for(binding.instance)
                .into_iter()
                .flatten()
        })
        .map(|offer| {
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
        })
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
