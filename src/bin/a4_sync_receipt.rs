// Copyright 2026 Mark AB (markik)
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::path::{Path, PathBuf};

use cleromancy::{
    CleromancyApp, CleromancyHost, CleromancySyncImport, CleromancySyncSelection, Reading,
    ReadingEngine, a0_fixture, export_sync_batch, import_sync_projection,
};
use graphshell::personal_sync::{PersonalGraphReplica, SyncRoster};
use muniment::MemoryBackend;
use personae::{IdentityProvider, InMemoryProvider};
use serde::Serialize;

const GRAPH: [u8; 32] = [0x44; 32];

#[derive(Serialize)]
struct SyncReceipt {
    schema: &'static str,
    evidence: &'static str,
    transport_evidence: &'static str,
    selection: CleromancySyncSelection,
    batch_digest: String,
    events: usize,
    operation: String,
    writer_subject: String,
    imported: CleromancySyncImport,
    replay_verified: bool,
    reading: Reading,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    pollster::block_on(run())
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments = std::env::args_os().skip(1);
    let html_path = arguments
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("receipts/a4-sync.html"));
    let json_path = arguments
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("receipts/a4-sync.json"));

    let (context, field) = a0_fixture();
    let reading = ReadingEngine::calculate(&context, &field)?;
    let mut source = CleromancyHost::empty(MemoryBackend::new());
    source.insert_reading(&context, &field, &reading)?;
    let selection = CleromancySyncSelection::ContextsAndReadings;
    let batch = export_sync_batch(&source, selection)?;

    let alice_identity = InMemoryProvider::from_seed([0x41; 32]);
    let bob_identity = InMemoryProvider::from_seed([0x42; 32]);
    let alice_subject = alice_identity.master_public_key().to_bytes();
    let roster = SyncRoster::new([alice_subject, bob_identity.master_public_key().to_bytes()]);
    let personal_selection = selection.personal_graph_selection();
    let mut alice = PersonalGraphReplica::for_identity(
        MemoryBackend::new(),
        GRAPH,
        &alice_identity,
        roster.clone(),
        personal_selection.clone(),
    )?;
    let bob = PersonalGraphReplica::for_identity(
        MemoryBackend::new(),
        GRAPH,
        &bob_identity,
        roster,
        personal_selection,
    )?;
    let operation = alice.author(batch.events.clone()).await?;
    if !bob.accept(&operation).await? {
        return Err("the receiving H7 replica did not insert the operation".into());
    }
    let projection = bob.projection().await?;
    if !projection.pending.is_empty() || !projection.conflicts.is_empty() {
        return Err("the receiving H7 projection was not settled".into());
    }

    let mut target = CleromancyHost::empty(MemoryBackend::new());
    let imported = import_sync_projection(&mut target, &projection, selection)?;
    let synced_reading = readings(&target)
        .into_iter()
        .next()
        .ok_or("the synced projection contained no reading")?;
    if target.replay_reading(&synced_reading)? != synced_reading {
        return Err("the synced reading did not replay".into());
    }
    let receipt = SyncReceipt {
        schema: "cleromancy.proof/a4-personal-sync-v2",
        evidence: "signed operation accepted by an independent Graphshell H7 replica",
        transport_evidence: "in-memory operation exchange; resident LogSync transport not exercised",
        selection,
        batch_digest: batch.digest,
        events: batch.events.len(),
        operation: hex32(operation.hash.as_bytes()),
        writer_subject: hex32(&alice_subject),
        imported,
        replay_verified: true,
        reading: synced_reading,
    };
    let mut app = CleromancyApp::new(target);
    write(&html_path, app.receipt_html()?.as_bytes())?;
    write(&json_path, &serde_json::to_vec_pretty(&receipt)?)?;
    println!(
        "accepted {} signed H7 events from {}; imported {} context, {} field, and {} reading; replay passed; wrote {} and {}",
        receipt.events,
        &receipt.writer_subject[..12],
        receipt.imported.contexts,
        receipt.imported.fields,
        receipt.imported.readings,
        html_path.display(),
        json_path.display()
    );
    Ok(())
}

fn readings(host: &CleromancyHost<MemoryBackend>) -> Vec<Reading> {
    host.graph()
        .nodes()
        .filter_map(|(key, _)| {
            host.facet_value(key, cleromancy::host::READING_FACET)
                .and_then(|value| serde_json::from_value(value.clone()).ok())
        })
        .collect()
}

fn hex32(bytes: &[u8; 32]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn write(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent().filter(|path| !path.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, bytes)
}
