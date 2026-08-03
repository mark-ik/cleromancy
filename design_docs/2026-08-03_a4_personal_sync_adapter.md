# Cleromancy A4: selected personal-sync adapter

**Date:** 2026-08-03
**Scope:** make sealed Cleromancy truth authorable and materializable through
Graphshell H7 without creating another replication system.

## Boundary

Graphshell H7 already owns Personae-derived writers, roster admission, causal
operation storage, conflict detection, LogSync, and the resident p2panda
transport. Cleromancy owns only the mapping between its domain graph and H7's
`PersonalGraphEvent` vocabulary.

`export_sync_batch` is pure and deterministic. It does not sign, store,
transmit, or grant anything. The caller gives its events to an admitted
`PersonalGraphReplica` or resident `PersonalSyncHost`. `import_sync_projection`
accepts H7's already-materialized projection, validates all selected domain
facets, then merges contexts, fields, and readings into the local Cleromancy
graph.

The `personal-sync` Cargo feature is optional. Runtime selection defaults to
`Off` and offers three settings:

- `Off` exports nothing;
- `Contexts` exports context nodes and `cleromancy.context/v1` facets;
- `ContextsAndReadings` also exports exact candidate-field nodes, reading
  nodes, their facets, and `GeneratedFrom` relations.

Reading sync includes contexts and fields by construction. A receipt whose
bound context or field is absent would preserve an answer while discarding
part of its workings.

## Privacy

Clotho's entropy source and raw entropy never enter graph sync. Cast receipts
do carry their deliberately disclosed bounded sample and nonce. Context facts,
questions, labels, and tags can still be personally sensitive.

H7 authenticates writers and encrypts its network transport, but A4 makes no
encrypted-storage-at-rest claim for the retained operation log. Sync remains an
explicit user setting, suitable only for the personal devices admitted to that
graph. Cleromancy does not infer consent from the existence of a Personae
identity or paired peer.

## Projection rules

Every exported node must retain its canonical `cleromancy://` address and the
Mere UUID derived from that address. Export order is stable: context nodes,
field nodes, reading nodes, then reading-to-context and reading-to-field
relations. Tags and nodes are sorted, and the complete versioned batch receives
a BLAKE3 digest. A5 advances that wrapper to `cleromancy.sync-batch/v2` when it
adds required field provenance.

Import validates the complete selected projection before changing local truth.
It refuses:

- operations waiting for missing causal history;
- unresolved concurrent values for any selected Cleromancy facet;
- malformed context, field, or reading facets;
- node IDs or addresses which do not match their domain content;
- readings whose bound context or exact candidate field is absent.

A4 imports additions and idempotent updates. It does not import deletion. A
generic personal-graph removal therefore cannot erase a local Cleromancy
reading through this adapter.

## Evidence

The focused proof uses two independent memory-backed H7 replicas with distinct
Personae roots. One authors the deterministic Cleromancy batch as a signed,
attested operation; the other admits it through the roster and materializes the
projection. A fresh Cleromancy host imports that projection and replays the
reading. A separate concurrent-writer test produces a real H7 facet conflict
and proves import leaves the target graph untouched.

This is signed-operation and materialization evidence. It is not a
Cleromancy-specific LogSync or physical-network run. The live Graphshell H7
donor has its own two-device receipt, but A4 does not substitute that receipt
for a resident Cleromancy composition test.

## Acceptance

1. The default selection exports zero events.
2. A full batch contains one context, one field, one reading, and their
   provenance, with a stable digest.
3. An independent admitted H7 replica accepts and attributes the signed batch.
4. Imported truth exports back to byte-equivalent events and the reading
   replays without contacting its source replica.
5. Concurrent Cleromancy facet values are refused before local mutation.
6. The HTML and JSON proof receipts are byte-stable across fresh runs.

## Stop rule

Stop before resident `PersonalSyncHost` lifecycle, pairing settings, automatic
publish queues, deletion propagation, encrypted sync storage, and a new
physical two-device run. Tarot, astrology, generated interpretation, and the
generic app plug-in SDK remain separate product slices.
