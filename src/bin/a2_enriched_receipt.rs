// Copyright 2026 Mark AB (markik)
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use cleromancy::{CleromancyApp, CleromancyHost, ReadingEngine, a2_fixture};
use graphshell_stdio::StdioCarrier;
use muniment::MemoryBackend;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments = env::args_os().skip(1);
    let endpoint = arguments
        .next()
        .map(PathBuf::from)
        .ok_or("usage: a2_enriched_receipt <graphshell-endpoint> [html] [json]")?;
    let html_path = arguments
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("receipts/a2-turnstone.html"));
    let json_path = arguments
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("receipts/a2-turnstone-reading.json"));

    let (context, field) = a2_fixture();
    let host = CleromancyHost::empty(MemoryBackend::new());
    let mut app = CleromancyApp::new(host);
    let mut carrier = StdioCarrier::spawn(&endpoint, std::iter::empty::<&str>())?;
    let external = app.mount_external(&mut carrier, 0)?;
    let evidence = external.seal(&context)?;
    let source_cards = evidence.sources.len();
    carrier.shutdown()?;

    let reading = ReadingEngine::calculate_enriched(&context, &field, &evidence)?;
    let replayed = ReadingEngine::replay(&context, &field, &reading.receipt)?;
    if replayed != reading {
        return Err("offline replay changed the sealed reading".into());
    }
    app.host.insert_reading(&context, &reading)?;

    write(&html_path, app.receipt_html()?.as_bytes())?;
    write(&json_path, &serde_json::to_vec_pretty(&reading)?)?;
    let qualification = reading
        .receipt
        .enrichment
        .as_ref()
        .expect("A2 reading carries enrichment");
    println!(
        "sealed {source_cards} external cards as {}; additions {:?}; selected {}; offline replay passed; wrote {} and {}",
        evidence.evidence_digest,
        qualification.weight_additions,
        reading.candidate_id,
        html_path.display(),
        json_path.display()
    );
    Ok(())
}

fn write(path: &Path, bytes: &[u8]) -> Result<(), std::io::Error> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, bytes)
}
