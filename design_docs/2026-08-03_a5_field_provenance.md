# Cleromancy A5: first-class field provenance

**Date:** 2026-08-03
**Scope:** retain the complete candidate field required to understand and
replay each reading.

## Contradiction

A3 accepts caller-supplied fields and stores the resulting reading. A4 can
replicate that reading and its context. Before A5, the graph retained only the
field digest. A fresh process or peer still needed the original caller or
catalog to reconstruct the candidates. The arithmetic was sealed, but one of
its declared inputs was missing.

## Graph truth

A5 gives an exact `Field` snapshot its own `cleromancy.field/v1` facet and the
canonical address `cleromancy://field/{digest}`. The Mere UUID derives from the
address. Equal fields deduplicate by digest.

Every new reading has two `GeneratedFrom` provenance relations:

- reading to its context;
- reading to its exact candidate field.

The field contains the system identifier, rule identifier, ordered candidates,
base weights, tags, and authored interpretations used by the reading. It is a
sealed input, not an installed catalog or a claim that the named rule is true.

`CleromancyHost::insert_reading` replays and compares the reading before any
graph mutation. `CleromancyHost::replay_reading` resolves the context and field
by their receipt digests and then invokes the existing reading engine. A
missing dependency is an explicit error.

Graphshell projects field nodes as portable cards showing the system, rule,
and candidate weights. This makes the field visible in the orrery rather than
leaving the field digest as an unexplained string on the reading card.

## Intent and sync consequences

An accepted A3 `read`, `select`, or `roll` now persists the field supplied or
constructed for that command. The caller can disappear after acceptance.

A4 `ContextsAndReadings` now includes `cleromancy.field/v1`. Export adds field
nodes before readings and emits reading-to-field provenance. The batch wrapper
advances to `cleromancy.sync-batch/v2`. Import validates field facets,
canonical identities, field conflicts, and every reading/field binding before
changing local truth. `Contexts` still carries only contexts.

Fields may contain private or proprietary authored text. Selecting reading
sync therefore discloses the exact field to admitted personal devices along
with the reading. Selecting context-only sync does not disclose fields.

## Compatibility

The reading and receipt schemas do not change. `field_digest` already named
the required input. A5 supplies the missing graph object and strengthens host
insertion.

Previously persisted readings may lack field nodes. They remain readable and
projectable, but graph-resident replay reports the missing field. Automatic
migration cannot recover an arbitrary caller-supplied field from its digest
and is outside this slice.

## Acceptance

1. A Graphshell JSON intent creates context, field, and reading nodes.
2. The reading points to context and field through distinct provenance edges.
3. Replay succeeds after the caller's field payload and context are dropped.
4. A missing field produces an explicit dependency error.
5. Full personal sync carries one field and the imported reading replays using
   only the target graph.
6. A sync projection missing the field is rejected before target mutation.
7. The HTML and JSON A5 receipts are byte-stable.

## Stop rule

Stop before catalog installation, Tarot or astrology content, multi-card
spreads, correspondence packs, generated interpretation, a generic plug-in
SDK, or speculative recovery of legacy fields. The next catalog slice must use
the field node as its first real consumer rather than creating another source
of replay truth.
