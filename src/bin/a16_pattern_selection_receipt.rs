// Copyright 2026 Mark AB (markik)
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::collections::{BTreeMap, VecDeque};
use std::path::{Path, PathBuf};

use cleromancy::moirai::clotho::EntropySource;
use cleromancy::servitor::{Cap, Grant, Mode, Subject};
use cleromancy::{
    AstrologyChart, AstrologyFacts, AstrologyMoment, AstrologyPosition,
    AstrologyReadingConcurrenceIntentPayload, CREATE_CONCURRENCE_INTENT, CleromancyApp,
    CleromancyHost, Concurrence, Reading, ReadingEngine, ReadingError, ReadingSession, a0_fixture,
};
use graphshell_local::LocalCarrier;
use graphshell_protocol::{
    Carrier, CarrierRequestBody, CarrierResponseBody, IntentInvocation, IntentResult,
    ProjectionSnapshot, ResourceRequest,
};
use muniment::MemoryBackend;
use serde::Serialize;

#[derive(Serialize)]
struct PatternSelectionReceipt {
    schema: &'static str,
    selection_target: String,
    payload: AstrologyReadingConcurrenceIntentPayload,
    astrology_chart: AstrologyChart,
    astrology_facts: AstrologyFacts,
    tarot_reading: Reading,
    reading_session: ReadingSession,
    concurrence: Concurrence,
    concurrence_claim: &'static str,
    graph_nodes: usize,
    graph_relations: usize,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments = std::env::args_os().skip(1);
    let html_path = arguments
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("receipts/a16-pattern-selection.html"));
    let json_path = arguments
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("receipts/a16-pattern-selection.json"));

    let (context, field) = a0_fixture();
    let chart = AstrologyChart::new(
        "fixture-positions/v1",
        "fixture-engine",
        "fixture-ephemeris",
        AstrologyMoment::global("2026-08-05T12:00:00Z"),
        [
            AstrologyPosition::new("moon", 180_000, 1_000),
            AstrologyPosition::new("sun", 0, 0),
        ],
    )?;
    let facts = chart.facts(1_000)?;
    let reading = ReadingEngine::calculate(&context, &field)?;
    let mut entropy = FixedEntropy::new([0x16, 0x17]);
    let mut host = CleromancyHost::empty(MemoryBackend::new());
    host.insert_astrology_chart(&chart, 1_000)?;
    let session = host.record_reading_session_at_with_entropy(
        &context,
        &field,
        &reading,
        1_785_931_200_000,
        Some("a16-pattern-selection".to_string()),
        &mut entropy,
    )?;
    let subject = Subject::new([0x16; 32]);
    let mut app = CleromancyApp::new(host);
    app.bind_intent_subject(subject);
    app.servitors_mut().grant(Grant::new(
        subject,
        Cap::scope("cleromancy/intents")?,
        Mode::Write,
    ))?;
    let mut carrier = LocalCarrier::new(app, |_, _| Err("resume is not used".to_string()));

    let request = discover_request(&mut carrier)?;
    let first = snapshot(&mut carrier, &request)?;
    let selection_target = target_for_address(
        &mut carrier,
        &first,
        &format!("cleromancy://astrology/facts/{}", facts.digest()),
    )?;
    let action = action_for(&first, selection_target, CREATE_CONCURRENCE_INTENT)?;
    let payload_bytes = action.compose_payload(&BTreeMap::from([
        ("astrology_facts_digest".to_string(), facts.digest()),
        ("reading_session_id".to_string(), session.id.clone()),
    ]))?;
    let payload =
        serde_json::from_slice::<AstrologyReadingConcurrenceIntentPayload>(&payload_bytes)?;
    let result = request_intent(
        &mut carrier,
        IntentInvocation {
            session: first.session.clone(),
            target: selection_target,
            observed_epoch: first.scene.epoch,
            observed_revision: first.scene.revision,
            intent: CREATE_CONCURRENCE_INTENT.to_string(),
            payload: payload_bytes,
        },
    )?;
    if result != IntentResult::Accepted || carrier.take_notice().is_none() {
        return Err("bound pattern selection was not accepted and noticed".into());
    }
    let _second = snapshot(&mut carrier, &request)?;
    let concurrence =
        domain_values::<Concurrence>(carrier.endpoint(), cleromancy::CONCURRENCE_FACET)
            .into_iter()
            .next()
            .ok_or("accepted action did not save a concurrence")?;
    if carrier.endpoint().host.replay_concurrence(&concurrence)? != concurrence {
        return Err("saved concurrence did not replay".into());
    }
    let target_address = format!("cleromancy://astrology/facts/{}", facts.digest());
    let receipt = PatternSelectionReceipt {
        schema: "cleromancy.proof/a16-pattern-selection-v1",
        selection_target: target_address,
        payload,
        astrology_chart: chart,
        astrology_facts: facts,
        tarot_reading: reading,
        reading_session: session,
        concurrence,
        concurrence_claim: "consulted together; astrology did not qualify or cause the tarot cast",
        graph_nodes: carrier.endpoint().host.graph().nodes().count(),
        graph_relations: carrier.endpoint().host.graph().relations().count(),
    };
    let html = carrier.endpoint_mut().receipt_html()?;
    write(&html_path, html.as_bytes())?;
    write(&json_path, &serde_json::to_vec_pretty(&receipt)?)?;
    println!(
        "selected and saved one cross-system pattern occasion; wrote {} and {}",
        html_path.display(),
        json_path.display()
    );
    Ok(())
}

fn discover_request(
    carrier: &mut impl Carrier,
) -> Result<graphshell_protocol::ProjectionRequest, Box<dyn std::error::Error>> {
    match carrier.request(CarrierRequestBody::Discover)? {
        CarrierResponseBody::Descriptor(descriptor) => {
            Ok(descriptor.projections[0].request.clone())
        }
        response => Err(format!("unexpected discovery response: {response:?}").into()),
    }
}

fn snapshot(
    carrier: &mut impl Carrier,
    request: &graphshell_protocol::ProjectionRequest,
) -> Result<ProjectionSnapshot, Box<dyn std::error::Error>> {
    match carrier.request(CarrierRequestBody::Snapshot(request.clone()))? {
        CarrierResponseBody::Snapshot(snapshot) => Ok(*snapshot),
        response => Err(format!("unexpected snapshot response: {response:?}").into()),
    }
}

fn target_for_address(
    carrier: &mut impl Carrier,
    snapshot: &ProjectionSnapshot,
    address: &str,
) -> Result<sceno::InstanceId, Box<dyn std::error::Error>> {
    for binding in &snapshot.presentation.bindings {
        let Some(offer) = snapshot
            .presentation
            .offers_for(binding.instance)
            .and_then(|offers| offers.first())
        else {
            continue;
        };
        let response = carrier.request(CarrierRequestBody::Resource(ResourceRequest {
            session: snapshot.session.clone(),
            resource: offer.resource,
        }))?;
        let CarrierResponseBody::Resource(response) = response else {
            return Err("expected a portable-card resource".into());
        };
        let card: graphshell_protocol::PortableCardV1 = serde_json::from_slice(&response.bytes)?;
        if card
            .values
            .iter()
            .any(|value| value.label == "Address" && value.value == address)
        {
            return Ok(binding.instance);
        }
    }
    Err(format!("no portable card for {address}").into())
}

fn action_for(
    snapshot: &ProjectionSnapshot,
    target: sceno::InstanceId,
    intent: &str,
) -> Result<graphshell_protocol::AdvertisedAction, Box<dyn std::error::Error>> {
    snapshot
        .presentation
        .offers_for(target)
        .into_iter()
        .flatten()
        .flat_map(|offer| &offer.semantics.actions)
        .find(|action| action.intent.0 == intent)
        .cloned()
        .ok_or_else(|| format!("target does not advertise {intent}").into())
}

fn request_intent(
    carrier: &mut impl Carrier,
    intent: IntentInvocation,
) -> Result<IntentResult, Box<dyn std::error::Error>> {
    match carrier.request(CarrierRequestBody::Intent(intent))? {
        CarrierResponseBody::Intent(result) => Ok(result),
        response => Err(format!("unexpected intent response: {response:?}").into()),
    }
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

fn write(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent().filter(|path| !path.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, bytes)
}
