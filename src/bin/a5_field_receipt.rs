// Copyright 2026 Mark AB (markik)
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::path::{Path, PathBuf};

use cleromancy::servitor::{Cap, Grant, Mode, Subject};
use cleromancy::{
    CleromancyApp, CleromancyHost, READ_INTENT, READ_SCOPE, Reading, ReadingIntentPayload,
    a0_fixture,
};
use graphshell_local::LocalCarrier;
use graphshell_protocol::{
    Carrier, CarrierRequestBody, CarrierResponseBody, IntentInvocation, IntentResult,
    ProjectionSnapshot,
};
use mere::kernel::graph::{ProvenanceSubKind, RelationKind};
use muniment::MemoryBackend;
use serde::Serialize;

#[derive(Serialize)]
struct FieldReceipt {
    schema: &'static str,
    origin: &'static str,
    field_digest: String,
    field_address: String,
    graph_nodes: usize,
    provenance_relations: usize,
    replay_verified: bool,
    reading: Reading,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments = std::env::args_os().skip(1);
    let html_path = arguments
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("receipts/a5-field.html"));
    let json_path = arguments
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("receipts/a5-field.json"));

    let (context, field) = a0_fixture();
    let field_digest = field.digest();
    let field_address = format!("cleromancy://field/{field_digest}");
    let mut host = CleromancyHost::empty(MemoryBackend::new());
    host.insert_context(&context)?;
    let subject = Subject::new([0x55; 32]);
    let mut app = CleromancyApp::new(host);
    app.bind_intent_subject(subject);
    app.servitors_mut()
        .grant(Grant::new(subject, Cap::scope(READ_SCOPE)?, Mode::Write))?;
    let mut carrier = LocalCarrier::new(app, |_, _| Err("resume is not used".to_string()));

    let request = discover_request(&mut carrier)?;
    let snapshot = snapshot(&mut carrier, &request)?;
    let target = context_target(&snapshot)?;
    let payload = ReadingIntentPayload::read(field);
    let result = request_intent(
        &mut carrier,
        IntentInvocation {
            session: snapshot.session,
            target,
            observed_epoch: snapshot.scene.epoch,
            observed_revision: snapshot.scene.revision,
            intent: READ_INTENT.to_string(),
            payload: serde_json::to_vec(&payload)?,
        },
    )?;
    if result != IntentResult::Accepted {
        return Err(format!("A5 read was not accepted: {result:?}").into());
    }
    drop(payload);
    drop(context);

    let reading = readings(carrier.endpoint())
        .into_iter()
        .next()
        .ok_or("the accepted intent appended no reading")?;
    if carrier.endpoint().host.replay_reading(&reading)? != reading {
        return Err("graph-resident replay changed the reading".into());
    }
    if carrier
        .endpoint()
        .host
        .graph()
        .get_node_by_url(&field_address)
        .is_none()
    {
        return Err("the candidate field was not retained as graph truth".into());
    }
    let graph_nodes = carrier.endpoint().host.graph().nodes().count();
    let provenance_relations = carrier
        .endpoint()
        .host
        .graph()
        .relations()
        .filter(|relation| {
            relation.kind == RelationKind::Provenance(ProvenanceSubKind::GeneratedFrom)
        })
        .count();
    let receipt = FieldReceipt {
        schema: "cleromancy.proof/a5-field-provenance-v1",
        origin: "Graphshell LocalCarrier JSON intent",
        field_digest,
        field_address,
        graph_nodes,
        provenance_relations,
        replay_verified: true,
        reading,
    };
    let html = carrier.endpoint_mut().receipt_html()?;
    write(&html_path, html.as_bytes())?;
    write(&json_path, &serde_json::to_vec_pretty(&receipt)?)?;
    println!(
        "retained {} graph nodes and {} provenance relations; graph-resident replay passed; wrote {} and {}",
        receipt.graph_nodes,
        receipt.provenance_relations,
        html_path.display(),
        json_path.display()
    );
    Ok(())
}

fn discover_request(
    carrier: &mut impl Carrier,
) -> Result<graphshell_protocol::ProjectionRequest, Box<dyn std::error::Error>> {
    match carrier
        .request(CarrierRequestBody::Discover)
        .map_err(std::io::Error::other)?
    {
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
    match carrier
        .request(CarrierRequestBody::Snapshot(request.clone()))
        .map_err(std::io::Error::other)?
    {
        CarrierResponseBody::Snapshot(snapshot) => Ok(*snapshot),
        response => Err(format!("unexpected snapshot response: {response:?}").into()),
    }
}

fn context_target(
    snapshot: &ProjectionSnapshot,
) -> Result<sceno::InstanceId, Box<dyn std::error::Error>> {
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
        .map(|binding| binding.instance)
        .ok_or_else(|| "snapshot advertises no context command target".into())
}

fn request_intent(
    carrier: &mut impl Carrier,
    intent: IntentInvocation,
) -> Result<IntentResult, Box<dyn std::error::Error>> {
    match carrier
        .request(CarrierRequestBody::Intent(intent))
        .map_err(std::io::Error::other)?
    {
        CarrierResponseBody::Intent(result) => Ok(result),
        response => Err(format!("unexpected intent response: {response:?}").into()),
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

fn write(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent().filter(|path| !path.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, bytes)
}
