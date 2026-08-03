# Cleromancy A0: Graphshell product cut

**Date:** 2026-08-02
**Scope:** `repos/cleromancy` only. Mere and Turnstone are consumers/donors and
remain unchanged in this slice.

## Boundary

Cleromancy owns context, candidate fields, selection rules, readings, receipts,
and their local graph. It implements Graphshell endpoint traits beside that
truth and mounts its own endpoint through `graphshell-client`, matching the
local/remote projection discipline proved by `mere/ports/graphshell`.

The Graphshell port is a donor and a narrow view dependency. Portable protocol,
client, and endpoint contracts remain in Mere. Turnstone enrichment will arrive
as a mounted source-owned projection rather than as imported Turnstone state.

Servitor is composed directly into `CleromancyApp`. A0 exposes its existing
`Gate` and `GrantTable` and proves capability coverage. Petition lowering into
the reading graph is deliberately outside A0.

## Data model

- `cleromancy.context/v1`: versioned facts and tags.
- `cleromancy.reading/v1`: selected candidate, interpretation, and receipt.
- reading nodes relate to their context through `GeneratedFrom` provenance.
- projection resources are memory-only and purge on revocation by default.

The selection receipt records the context and field digests, qualified weights,
algorithm, selected index and candidate, and the bounded sample for cast mode.
The event nonce distinguishes repeated casts. It proves replay and calculation,
not supernatural causation or the quality of an entropy source.

## Acceptance

1. Identical context, field, and rules produce byte-identical calculated
   readings.
2. Cast mode reads the operating-system CSPRNG and uses rejection sampling
   before weighted selection.
3. A cast replays exactly from its receipt without drawing again.
4. A Redb-backed graph reopens with byte-identical graph and facet truth.
5. A local Graphshell client resolves context and reading cards and the static
   receipt includes their disclosed relations.
6. `CleromancyApp::servitors()` exposes the real Servitor gate and authority
   table; a focused test proves a scoped grant covers only its declared access.

## Stop rule

Stop after A0. Turnstone projection enrichment, Graphshell `read`/`select`/`roll`
intents, astrology/ephemeris integration, tarot data, Wasm denizens, and H7
personal sync are follow-on slices.
