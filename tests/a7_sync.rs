// Copyright 2026 Mark AB (markik)
// SPDX-License-Identifier: MIT OR Apache-2.0

#![cfg(all(feature = "personal-sync", not(target_arch = "wasm32")))]

use cleromancy::{
    CleromancyHost, CleromancySyncSelection, ReadingEngine, ReadingSession, Reflection, a0_fixture,
    export_sync_batch, import_sync_projection,
};
use graphshell::personal_sync::{PersonalGraphReplica, SyncRoster};
use muniment::MemoryBackend;
use personae::{IdentityProvider, InMemoryProvider};

#[test]
fn selected_session_history_syncs_but_reflections_need_explicit_consent() {
    pollster::block_on(async {
        let (context, field) = a0_fixture();
        let reading = ReadingEngine::calculate(&context, &field).unwrap();
        let session = ReadingSession::single(
            1_735_689_600_000,
            "00000000000000110000000000000022",
            context.digest(),
            field.digest(),
            &reading.id,
            Some("a7-sync".to_string()),
        )
        .unwrap();
        let reflection = Reflection::new(
            &session.id,
            1_735_689_601_000,
            "00000000000000330000000000000044",
            "This note is a separately selected private fact.",
        )
        .unwrap();
        let mut source = CleromancyHost::empty(MemoryBackend::new());
        source
            .insert_session(&context, &field, std::slice::from_ref(&reading), &session)
            .unwrap();
        source.insert_reflection(&session, &reflection).unwrap();

        let readings_only =
            export_sync_batch(&source, CleromancySyncSelection::ContextsAndReadings).unwrap();
        assert_eq!(
            (
                readings_only.contexts,
                readings_only.fields,
                readings_only.readings,
                readings_only.sessions,
                readings_only.reflections,
            ),
            (1, 1, 1, 1, 0)
        );

        let selection = CleromancySyncSelection::ContextsReadingsAndReflections;
        let batch = export_sync_batch(&source, selection).unwrap();
        assert_eq!(
            (
                batch.contexts,
                batch.fields,
                batch.readings,
                batch.sessions,
                batch.reflections,
            ),
            (1, 1, 1, 1, 1)
        );

        let alice_identity = InMemoryProvider::from_seed([0x71; 32]);
        let bob_identity = InMemoryProvider::from_seed([0x72; 32]);
        let roster = SyncRoster::new([
            alice_identity.master_public_key().to_bytes(),
            bob_identity.master_public_key().to_bytes(),
        ]);
        let mut alice = PersonalGraphReplica::for_identity(
            MemoryBackend::new(),
            [0x77; 32],
            &alice_identity,
            roster.clone(),
            selection.personal_graph_selection(),
        )
        .unwrap();
        let bob = PersonalGraphReplica::for_identity(
            MemoryBackend::new(),
            [0x77; 32],
            &bob_identity,
            roster,
            selection.personal_graph_selection(),
        )
        .unwrap();
        let operation = alice.author(batch.events.clone()).await.unwrap();
        assert!(bob.accept(&operation).await.unwrap());
        let projection = bob.projection().await.unwrap();

        let mut target = CleromancyHost::empty(MemoryBackend::new());
        let imported = import_sync_projection(&mut target, &projection, selection).unwrap();
        assert_eq!(
            (
                imported.contexts,
                imported.fields,
                imported.readings,
                imported.sessions,
                imported.reflections,
            ),
            (1, 1, 1, 1, 1)
        );
        assert_eq!(target.replay_session(&session).unwrap(), vec![reading]);
        let round_trip = export_sync_batch(&target, selection).unwrap();
        assert_eq!(round_trip.events, batch.events);
        assert_eq!(round_trip.digest, batch.digest);
    });
}
