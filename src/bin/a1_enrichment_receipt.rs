// Copyright 2026 Mark AB (markik)
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::path::{Path, PathBuf};

use cleromancy::{CleromancyApp, CleromancyHost, ReadingEngine, a1_fixture};
use graphshell_stdio::StdioCarrier;
use muniment::MemoryBackend;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments = std::env::args_os().skip(1);
    let endpoint = arguments
        .next()
        .map(PathBuf::from)
        .ok_or("usage: a1_enrichment_receipt <endpoint> [html] [json]")?;
    let html_path = arguments
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("receipts/a1-enrichment.html"));
    let report_path = arguments
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("receipts/a1-enrichment.json"));

    let (context, field) = a1_fixture();
    let reading = ReadingEngine::calculate(&context, &field)?;
    let mut host = CleromancyHost::empty(MemoryBackend::new());
    host.insert_reading(&context, &field, &reading)?;
    let mut app = CleromancyApp::new(host);
    let local_cards = app.mount_local()?.len();

    let mut carrier = StdioCarrier::spawn(&endpoint, std::iter::empty::<&str>())?;
    let external = app.mount_external(&mut carrier, 0)?;
    let report = external.correlate(&context)?;
    let html = app.enrichment_receipt_html(&external, &report)?;
    write(&html_path, html.as_bytes())?;
    write(&report_path, &serde_json::to_vec_pretty(&report)?)?;
    carrier.shutdown()?;

    println!(
        "mounted {local_cards} local and {} external cards; wrote {} and {}",
        external.presentations.len(),
        html_path.display(),
        report_path.display()
    );
    Ok(())
}

fn write(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, bytes)
}
