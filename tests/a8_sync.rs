// Copyright 2026 Mark AB (markik)
// SPDX-License-Identifier: MIT OR Apache-2.0

#![cfg(all(feature = "personal-sync", not(target_arch = "wasm32")))]

use std::collections::VecDeque;

use cleromancy::moirai::clotho::EntropySource;
use cleromancy::{
    CleromancyHost, CleromancySyncSelection, ReadingError, TarotPack, TarotQualification,
    export_sync_batch, import_sync_projection,
};
use graphshell::personal_sync::{PersonalGraphReplica, SyncRoster};
use muniment::MemoryBackend;
use personae::{IdentityProvider, InMemoryProvider};

#[test]
fn authored_spread_syncs_as_selected_graph_truth() {
    pollster::block_on(async {
        let context =
            cleromancy::ContextSnapshot::new("A turning point", "cleromancy.a8-sync-context/v1");
        let field = TarotPack::rws_major_arcana().field(TarotQualification::Uniform);
        let mut source = CleromancyHost::empty(MemoryBackend::new());
        let mut entropy =
            FixedEntropy::new([0, 0x11, 0x22, 1, 0x33, 0x44, 2, 0x55, 0x66, 0x77, 0x88]);
        let (session, spread, readings) = source
            .record_three_card_spread_at_with_entropy(
                &context,
                &field,
                1_753_000_001_000,
                Some("a8-sync".to_string()),
                &mut entropy,
            )
            .unwrap();
        let selection = CleromancySyncSelection::ContextsAndReadings;
        let batch = export_sync_batch(&source, selection).unwrap();
        assert_eq!((batch.sessions, batch.spreads), (1, 1));

        let alice_identity = InMemoryProvider::from_seed([0x81; 32]);
        let bob_identity = InMemoryProvider::from_seed([0x82; 32]);
        let roster = SyncRoster::new([
            alice_identity.master_public_key().to_bytes(),
            bob_identity.master_public_key().to_bytes(),
        ]);
        let mut alice = PersonalGraphReplica::for_identity(
            MemoryBackend::new(),
            [0x83; 32],
            &alice_identity,
            roster.clone(),
            selection.personal_graph_selection(),
        )
        .unwrap();
        let bob = PersonalGraphReplica::for_identity(
            MemoryBackend::new(),
            [0x83; 32],
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
        assert_eq!((imported.sessions, imported.spreads), (1, 1));
        assert_eq!(target.replay_three_card_spread(&spread).unwrap(), readings);
        let round_trip = export_sync_batch(&target, selection).unwrap();
        assert_eq!(round_trip.events, batch.events);
        assert_eq!(round_trip.digest, batch.digest);
        assert_eq!(target.replay_session(&session).unwrap().len(), 3);
    });
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
