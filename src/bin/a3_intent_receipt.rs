// Copyright 2026 Mark AB (markik)
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::path::{Path, PathBuf};

use cleromancy::servitor::{Cap, Grant, Mode, Subject};
use cleromancy::{
    CleromancyApp, CleromancyHost, READ_INTENT, READ_SCOPE, Reading, ReadingEngine,
    ReadingIntentPayload, a0_fixture,
};
use graphshell_local::LocalCarrier;
use graphshell_protocol::{
    Carrier, CarrierNotice, CarrierRequestBody, CarrierResponseBody, IntentInvocation,
    IntentResult, ProjectionRequest, ProjectionSnapshot,
};
use muniment::MemoryBackend;
use serde::Serialize;

#[derive(Serialize)]
struct IntentReceipt {
    schema: &'static str,
    carrier: &'static str,
    identity_binding: &'static str,
    granted_scope: &'static str,
    intent: &'static str,
    result: Reading,
    notice: CarrierNotice,
    servitor_audit_revision: u64,
    replay_verified: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments = std::env::args_os().skip(1);
    let html_path = arguments
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("receipts/a3-intent.html"));
    let json_path = arguments
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("receipts/a3-intent.json"));

    let (context, field) = a0_fixture();
    let mut host = CleromancyHost::empty(MemoryBackend::new());
    host.insert_context(&context)?;
    let mut app = CleromancyApp::new(host);
    let subject = Subject::new([3; 32]);
    app.bind_intent_subject(subject);
    app.servitors_mut()
        .grant(Grant::new(subject, Cap::scope(READ_SCOPE)?, Mode::Write))?;

    let mut carrier = LocalCarrier::new(app, |_, _| Err("resume is not used".to_string()));
    let request = discover(&mut carrier)?;
    let snapshot = snapshot(&mut carrier, &request)?;
    let target = context_target(&snapshot)?;
    let invocation = IntentInvocation {
        session: snapshot.session.clone(),
        target,
        observed_epoch: snapshot.scene.epoch,
        observed_revision: snapshot.scene.revision,
        intent: READ_INTENT.to_string(),
        payload: serde_json::to_vec(&ReadingIntentPayload::read(field.clone()))?,
    };
    let result = carrier.request(CarrierRequestBody::Intent(invocation))?;
    if result != CarrierResponseBody::Intent(IntentResult::Accepted) {
        return Err(format!("A3 read was not accepted: {result:?}").into());
    }
    let notice = carrier
        .take_notice()
        .ok_or("accepted A3 read did not emit a revision notice")?;
    let reading = readings(carrier.endpoint())?
        .into_iter()
        .next()
        .ok_or("accepted A3 read did not append a reading")?;
    if ReadingEngine::replay(&context, &field, &reading.receipt)? != reading {
        return Err("A3 reading did not replay".into());
    }
    let receipt = IntentReceipt {
        schema: "cleromancy.proof/a3-intent-v1",
        carrier: "graphshell-local JSON wire round-trip",
        identity_binding: "subject supplied by the containing host, never the payload",
        granted_scope: READ_SCOPE,
        intent: READ_INTENT,
        result: reading,
        notice,
        servitor_audit_revision: carrier.endpoint().servitors().audit().revision(),
        replay_verified: true,
    };
    let html = carrier.endpoint_mut().receipt_html()?;
    write(&html_path, html.as_bytes())?;
    write(&json_path, &serde_json::to_vec_pretty(&receipt)?)?;
    println!(
        "accepted {} through the JSON wire; Servitor audit revision {}; replay passed; wrote {} and {}",
        receipt.intent,
        receipt.servitor_audit_revision,
        html_path.display(),
        json_path.display()
    );
    Ok(())
}

fn discover(carrier: &mut impl Carrier) -> Result<ProjectionRequest, Box<dyn std::error::Error>> {
    match carrier.request(CarrierRequestBody::Discover)? {
        CarrierResponseBody::Descriptor(descriptor) => descriptor
            .projections
            .into_iter()
            .next()
            .map(|offer| offer.request)
            .ok_or_else(|| "Cleromancy advertised no projection".into()),
        response => Err(format!("unexpected discovery response: {response:?}").into()),
    }
}

fn snapshot(
    carrier: &mut impl Carrier,
    request: &ProjectionRequest,
) -> Result<ProjectionSnapshot, Box<dyn std::error::Error>> {
    match carrier.request(CarrierRequestBody::Snapshot(request.clone()))? {
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
        .ok_or_else(|| "bound projection did not advertise its context intents".into())
}

fn readings(
    app: &CleromancyApp<MemoryBackend>,
) -> Result<Vec<Reading>, Box<dyn std::error::Error>> {
    app.host
        .graph()
        .nodes()
        .filter_map(|(key, _)| app.host.facet_value(key, cleromancy::host::READING_FACET))
        .map(|value| serde_json::from_value(value.clone()).map_err(Into::into))
        .collect()
}

fn write(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent().filter(|path| !path.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, bytes)
}
