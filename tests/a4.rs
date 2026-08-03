// Copyright 2026 Mark AB (markik)
// SPDX-License-Identifier: MIT OR Apache-2.0

#![cfg(all(feature = "personal-sync", not(target_arch = "wasm32")))]

use cleromancy::{
    CleromancyHost, CleromancySyncError, CleromancySyncSelection, Reading, ReadingEngine,
    a0_fixture, export_sync_batch, import_sync_projection,
};
use graphshell::personal_sync::{PersonalGraphEvent, PersonalGraphReplica, SyncRoster};
use muniment::MemoryBackend;
use personae::{IdentityProvider, InMemoryProvider};

const GRAPH: [u8; 32] = [0x44; 32];

#[test]
fn signed_personal_replicas_rematerialize_a_replayable_reading() {
    pollster::block_on(async {
        let (context, field) = a0_fixture();
        let reading = ReadingEngine::calculate(&context, &field).unwrap();
        let mut source = CleromancyHost::empty(MemoryBackend::new());
        source.insert_reading(&context, &field, &reading).unwrap();

        let off = export_sync_batch(&source, CleromancySyncSelection::Off).unwrap();
        assert!(off.is_empty(), "personal sync is opt-in");
        let selection = CleromancySyncSelection::ContextsAndReadings;
        let batch = export_sync_batch(&source, selection).unwrap();
        assert_eq!((batch.contexts, batch.fields, batch.readings), (1, 1, 1));

        let alice_identity = InMemoryProvider::from_seed([0x11; 32]);
        let bob_identity = InMemoryProvider::from_seed([0x22; 32]);
        let alice_subject = alice_identity.master_public_key().to_bytes();
        let roster = SyncRoster::new([alice_subject, bob_identity.master_public_key().to_bytes()]);
        let personal_selection = selection.personal_graph_selection();
        let mut alice = PersonalGraphReplica::for_identity(
            MemoryBackend::new(),
            GRAPH,
            &alice_identity,
            roster.clone(),
            personal_selection.clone(),
        )
        .unwrap();
        let bob = PersonalGraphReplica::for_identity(
            MemoryBackend::new(),
            GRAPH,
            &bob_identity,
            roster,
            personal_selection,
        )
        .unwrap();

        let operation = alice.author(batch.events.clone()).await.unwrap();
        assert!(bob.accept(&operation).await.unwrap());
        let projection = bob.projection().await.unwrap();
        assert!(projection.pending.is_empty());
        assert!(projection.conflicts.is_empty());
        assert_eq!(projection.writers.len(), 1);
        assert_eq!(projection.writers[0].stable_subject, alice_subject);

        let mut target = CleromancyHost::empty(MemoryBackend::new());
        let imported = import_sync_projection(&mut target, &projection, selection).unwrap();
        assert_eq!(
            (imported.contexts, imported.fields, imported.readings),
            (1, 1, 1)
        );
        let imported_reading = readings(&target).pop().unwrap();
        assert_eq!(
            target.replay_reading(&imported_reading).unwrap(),
            imported_reading
        );

        let round_trip = export_sync_batch(&target, selection).unwrap();
        assert_eq!(round_trip.events, batch.events);
        assert_eq!(round_trip.digest, batch.digest);
    });
}

#[test]
fn concurrent_cleromancy_facet_values_are_not_silently_imported() {
    pollster::block_on(async {
        let (context, field) = a0_fixture();
        let reading = ReadingEngine::calculate(&context, &field).unwrap();
        let mut source = CleromancyHost::empty(MemoryBackend::new());
        source.insert_reading(&context, &field, &reading).unwrap();
        let selection = CleromancySyncSelection::ContextsAndReadings;
        let batch = export_sync_batch(&source, selection).unwrap();
        let mut altered = batch.events.clone();
        let value = altered
            .iter_mut()
            .find_map(|event| match event {
                PersonalGraphEvent::SetFacet { facet, value, .. }
                    if facet == cleromancy::host::CONTEXT_FACET =>
                {
                    Some(value)
                }
                _ => None,
            })
            .unwrap();
        let mut changed_context: cleromancy::ContextSnapshot =
            serde_json::from_value(value.clone()).unwrap();
        changed_context.label = "A conflicting description".to_string();
        *value = serde_json::to_value(changed_context).unwrap();

        let alice_identity = InMemoryProvider::from_seed([0x31; 32]);
        let bob_identity = InMemoryProvider::from_seed([0x32; 32]);
        let roster = SyncRoster::new([
            alice_identity.master_public_key().to_bytes(),
            bob_identity.master_public_key().to_bytes(),
        ]);
        let personal_selection = selection.personal_graph_selection();
        let mut alice = PersonalGraphReplica::for_identity(
            MemoryBackend::new(),
            GRAPH,
            &alice_identity,
            roster.clone(),
            personal_selection.clone(),
        )
        .unwrap();
        let mut bob = PersonalGraphReplica::for_identity(
            MemoryBackend::new(),
            GRAPH,
            &bob_identity,
            roster,
            personal_selection,
        )
        .unwrap();
        let alice_operation = alice.author(batch.events).await.unwrap();
        let bob_operation = bob.author(altered).await.unwrap();
        assert!(alice.accept(&bob_operation).await.unwrap());
        assert!(bob.accept(&alice_operation).await.unwrap());

        let projection = alice.projection().await.unwrap();
        assert!(
            projection
                .conflicts
                .iter()
                .any(|conflict| conflict.target.ends_with(cleromancy::host::CONTEXT_FACET))
        );
        let mut target = CleromancyHost::empty(MemoryBackend::new());
        assert!(matches!(
            import_sync_projection(&mut target, &projection, selection),
            Err(CleromancySyncError::Conflict(target))
                if target.ends_with(cleromancy::host::CONTEXT_FACET)
        ));
        assert!(target.is_empty());
    });
}

#[test]
fn reading_without_its_field_is_rejected_before_import() {
    pollster::block_on(async {
        let (context, field) = a0_fixture();
        let reading = ReadingEngine::calculate(&context, &field).unwrap();
        let mut source = CleromancyHost::empty(MemoryBackend::new());
        source.insert_reading(&context, &field, &reading).unwrap();
        let selection = CleromancySyncSelection::ContextsAndReadings;
        let mut events = export_sync_batch(&source, selection).unwrap().events;
        events.retain(|event| {
            !matches!(
                event,
                PersonalGraphEvent::SetFacet { facet, .. }
                    if facet == cleromancy::host::FIELD_FACET
            )
        });

        let identity = InMemoryProvider::from_seed([0x51; 32]);
        let roster = SyncRoster::new([identity.master_public_key().to_bytes()]);
        let mut replica = PersonalGraphReplica::for_identity(
            MemoryBackend::new(),
            GRAPH,
            &identity,
            roster,
            selection.personal_graph_selection(),
        )
        .unwrap();
        replica.author(events).await.unwrap();
        let projection = replica.projection().await.unwrap();

        let mut target = CleromancyHost::empty(MemoryBackend::new());
        assert!(matches!(
            import_sync_projection(&mut target, &projection, selection),
            Err(CleromancySyncError::MissingField { reading: id, .. }) if id == reading.id
        ));
        assert!(target.is_empty());
    });
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
