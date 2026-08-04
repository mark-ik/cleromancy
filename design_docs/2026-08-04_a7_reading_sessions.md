# Cleromancy A7: saved reading occasions

**Date:** 2026-08-04
**Scope:** distinguish a sealed result from the occasion on which someone or
an app asked for it, without changing the reading receipt or introducing a
spread language.

## Contradiction

Before A7, `Reading` named both an auditable selected result and a saved
occasion. That conflation erased a meaningful distinction. A calculated
question asked twice resolves to the same content-addressed result, so the
graph could retain only one node. There was no time, no history of repeated
consultation, no place for positions, and no way for a Graphshell caller to
find the exact accepted result after a resnapshot.

## Model

`cleromancy.reading-session/v1` is a host-minted immutable event. It records:

- local timestamp and a separate CSPRNG event nonce;
- the exact context and field digests;
- ordered result placements, with one `focus` placement in A7; and
- an optional opaque client token for post-acceptance correlation.

The session identifier commits to every one of those values. A calculated
result may therefore appear in multiple distinct sessions, while its sealed
receipt and reading node remain deduplicated. Production timestamps use the
local system clock. `record_reading_session_at_with_entropy` admits a supplied
timestamp and entropy source for deterministic tests and imports.

`cleromancy.reflection/v1` is a separate immutable node. It commits to its
session, timestamp, nonce, and note body. A later revision is a new reflection,
not a mutation of the sealed result or the saved occasion.

## Graph and projection

A session has `GeneratedFrom` edges to its context and field, plus ordered
`CollectionMember` edges to its retained results. A reflection has a semantic
`Elaborates` edge to its session, labelled `reflects on`. Sessions and
reflections project as ordinary Graphshell cards, so the orrery shows the
occasion as well as its result and inputs.

The existing `read`, `select`, and `roll` commands now append a session rather
than only inserting a reading. `IntentResult::Accepted` remains an
acknowledgement, not a result payload. A caller supplies a bounded, non-secret
client token and resnapshots; it can then identify the session card bearing
that token. The token must never be an access credential or other secret.

## Sync and privacy

`ContextsAndReadings` now includes contexts, fields, readings, and sessions.
It deliberately excludes reflections. The new
`ContextsReadingsAndReflections` choice includes separately attached reflective
notes. H7 remains responsible for identity, admission, causal history,
conflict detection, storage, and transport. Cleromancy still only projects and
imports selected domain facts.

Import validates all session and reflection identities and dependencies before
local mutation. A session without its exact context, field, or placed reading,
or a reflection without its selected session, is rejected. This does not add
deletion propagation, encrypted-at-rest local storage, pairing UI, or a
resident sync lifecycle.

## Acceptance

1. Two calculated consultations of one context and field produce two session
   nodes that point to one retained reading node.
2. Session replay resolves its context, field, and ordered results from the
   local graph and rechecks the sealed receipts.
3. A separately stored reflection cannot be altered without invalidating its
   identity.
4. Graphshell actions save sessions and expose the caller token after the
   announced revision is resnapshotted.
5. Reading-history sync carries sessions; reflection sync requires the more
   explicit selection; imports validate the whole subtree before mutation.
6. The fixed A7 HTML and JSON receipt is byte-stable.

## Limits and stop rule

A7 establishes a saved event model and graph projection, not a headed reading
editor. It has one `focus` placement only. Stop before a generic spread DSL,
multi-card draw rules, mutable journal editing, astrology snapshots,
correspondence packs, general app plug-ins, or generated interpretive prose.
The next real surface should make “New reading”, saved history, and reflection
entry visible without concealing the declared calculation.
