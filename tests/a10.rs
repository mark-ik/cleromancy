use cleromancy::servitor::{Cap, Grant, Mode, Subject};
use cleromancy::{
    COMPOSE_READING_INTENT, CleromancyApp, CleromancyHost, CompositionLayout,
    ReadingCompositionIntentPayload, ReadingSession, SelectionMode, ThreeCardSpread, a0_fixture,
};
use graphshell_local::LocalCarrier;
use graphshell_protocol::{
    Carrier, CarrierRequestBody, CarrierResponseBody, IntentInvocation, IntentResult,
    PortableCardV1, ProjectionSnapshot, ResourceRequest,
};
use muniment::MemoryBackend;

#[test]
fn generic_composer_saves_single_and_three_card_layouts() {
    let (context, field) = a0_fixture();
    let mut host = CleromancyHost::empty(MemoryBackend::new());
    host.insert_context(&context).unwrap();
    let subject = Subject::new([0x10; 32]);
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
    assert_eq!(
        request_intent(
            &mut carrier,
            invocation(
                &first,
                target,
                &ReadingCompositionIntentPayload::single_calculated(field.clone())
                    .with_client_token("a10-single"),
            ),
        ),
        IntentResult::Accepted
    );
    assert!(carrier.take_notice().is_some());

    let second = snapshot(&mut carrier, &request);
    assert_eq!(sessions(carrier.endpoint()).len(), 1);
    assert_eq!(
        sessions(carrier.endpoint())[0].client_token.as_deref(),
        Some("a10-single")
    );

    assert_eq!(
        request_intent(
            &mut carrier,
            invocation(
                &second,
                context_target(&second),
                &ReadingCompositionIntentPayload::three_card(field).with_client_token("a10-spread"),
            ),
        ),
        IntentResult::Accepted
    );
    assert!(carrier.take_notice().is_some());
    let third = snapshot(&mut carrier, &request);
    let sessions = sessions(carrier.endpoint());
    assert_eq!(sessions.len(), 2);
    assert_eq!(
        sessions
            .iter()
            .find(|session| session.client_token.as_deref() == Some("a10-spread"))
            .unwrap()
            .placements
            .len(),
        3
    );
    let spreads = domain_values::<ThreeCardSpread>(
        carrier.endpoint(),
        cleromancy::host::THREE_CARD_SPREAD_FACET,
    );
    assert_eq!(spreads.len(), 1);
    assert_eq!(
        carrier
            .endpoint()
            .host
            .replay_three_card_spread(&spreads[0])
            .unwrap()
            .len(),
        3
    );
    assert!(third.scene.revision > second.scene.revision);
}

#[test]
fn generic_composer_rejects_a_deterministic_three_card_request() {
    let (context, field) = a0_fixture();
    let mut host = CleromancyHost::empty(MemoryBackend::new());
    host.insert_context(&context).unwrap();
    let subject = Subject::new([0x11; 32]);
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
    let result = request_intent(
        &mut carrier,
        invocation(
            &first,
            context_target(&first),
            &ReadingCompositionIntentPayload::new(
                field,
                CompositionLayout::ThreeCard,
                SelectionMode::Calculated,
            ),
        ),
    );
    assert!(matches!(
        result,
        IntentResult::Rejected { reason } if reason.contains("requires cast")
    ));
    assert!(carrier.take_notice().is_none());
    assert_eq!(sessions(carrier.endpoint()).len(), 0);
}

#[test]
fn generic_composer_can_select_a_graph_resident_field() {
    let (context, field) = a0_fixture();
    let field_digest = field.digest();
    let mut host = CleromancyHost::empty(MemoryBackend::new());
    host.insert_context(&context).unwrap();
    host.insert_field(&field).unwrap();
    let subject = Subject::new([0x12; 32]);
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
    assert!(cards(&mut carrier, &first).iter().any(|card| {
        card.values
            .iter()
            .any(|value| value.label == "Digest" && value.value == field_digest)
    }));
    assert_eq!(
        request_intent(
            &mut carrier,
            invocation(
                &first,
                context_target(&first),
                &ReadingCompositionIntentPayload::stored(
                    field_digest,
                    CompositionLayout::Single,
                    SelectionMode::Calculated,
                )
                .with_client_token("a10-stored"),
            ),
        ),
        IntentResult::Accepted
    );
    assert!(carrier.take_notice().is_some());
    let sessions = sessions(carrier.endpoint());
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].client_token.as_deref(), Some("a10-stored"));
    assert_eq!(sessions[0].field_digest, field.digest());
}

#[test]
fn generic_composer_rejects_a_missing_graph_field() {
    let (context, field) = a0_fixture();
    let mut host = CleromancyHost::empty(MemoryBackend::new());
    host.insert_context(&context).unwrap();
    let subject = Subject::new([0x13; 32]);
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
    assert!(matches!(
        request_intent(
            &mut carrier,
            invocation(
                &first,
                context_target(&first),
                &ReadingCompositionIntentPayload::stored(
                    field.digest(),
                    CompositionLayout::Single,
                    SelectionMode::Calculated,
                ),
            ),
        ),
        IntentResult::Rejected { reason } if reason.contains("not found")
    ));
    assert!(carrier.take_notice().is_none());
    assert_eq!(sessions(carrier.endpoint()).len(), 0);
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
    payload: &ReadingCompositionIntentPayload,
) -> IntentInvocation {
    IntentInvocation {
        session: snapshot.session.clone(),
        target,
        observed_epoch: snapshot.scene.epoch,
        observed_revision: snapshot.scene.revision,
        intent: COMPOSE_READING_INTENT.to_string(),
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

fn sessions(app: &CleromancyApp<MemoryBackend>) -> Vec<ReadingSession> {
    domain_values(app, cleromancy::host::SESSION_FACET)
}
