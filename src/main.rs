// Copyright 2026 Mark AB (markik)
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::path::{Path, PathBuf};

use cleromancy::{CleromancyApp, CleromancyHost, ReadingEngine, a0_fixture};
use muniment::RedbBackend;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let store_path = store_path();
    if let Some(parent) = store_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let backend = RedbBackend::open(&store_path)?;
    let mut host = pollster::block_on(CleromancyHost::open(backend))?;
    if host.is_empty() {
        let (context, field) = a0_fixture();
        let calculated = ReadingEngine::calculate(&context, &field)?;
        let cast = ReadingEngine::cast(&context, &field)?;
        host.insert_reading(&context, &calculated)?;
        host.insert_reading(&context, &cast)?;
        let saved_at_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();
        pollster::block_on(host.persist(saved_at_secs))?;
    }

    let output = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("receipts/a0.html"));
    if let Some(parent) = output
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)?;
    }
    let mut app = CleromancyApp::new(host);
    std::fs::write(&output, app.receipt_html()?)?;
    println!("wrote {}", output.display());
    Ok(())
}

fn store_path() -> PathBuf {
    if let Some(root) = std::env::var_os("CLEROMANCY_ROOT") {
        return Path::new(&root).join("cleromancy.redb");
    }
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("cleromancy")
        .join("cleromancy.redb")
}
