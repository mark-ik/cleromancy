# Cleromancy

Cleromancy is a local-first reading and suggestion application built as a
Graphshell product cut over Mere. Facts, system calculations, and application
state form a context. A reading either follows declared rules deterministically
or casts securely across the same qualified field. Every result retains a
receipt that separates context, qualification, selection, and interpretation.

The internal reading organs are the three Fates:

- **Clotho** supplies fresh cryptographic randomness when a reading calls for it.
- **Lachesis** qualifies the candidate field and apportions a result.
- **Atropos** seals the reading and its replayable receipt.

Graphshell provides the projection and interoperability contracts. Cleromancy
owns its graph, facets, reading rules, and interpretations. Servitor remains
available through the application as the capability gate for resident helpers.

## A0

The first slice proves:

- calculated and cast readings over one context and field;
- OS-backed cryptographic draws with unbiased bounded selection;
- replay from the stored receipt;
- local Mere graph persistence through Muniment/Redb;
- Graphshell portable-card projection and a static HTML receipt;
- direct access to Servitor's existing gate and authority table.

Run the proof wall:

```powershell
cargo test
cargo run -- receipts/a0.html
```

`CLEROMANCY_ROOT` overrides the local data directory used by the binary.

Turnstone enrichment, outward reading intents, and personal sync begin after
A0. Private context and Clotho's secrets stay outside the secret-free graph
sync lane until Cleromancy has a sealed payload path.

## A1

A1 mounts an external Graphshell projection beside the local reading graph and
computes a disclosed lexical correlation report over its portable cards.
Turnstone remains the owner of its projected graph. Cleromancy stores neither
the remote scene nor the report in reading truth, and the report does not alter
selection weights. A2 provides the receipt schema which can seal the external
evidence used by a reading.

The cross-process receipt accepts any Graphshell stdio endpoint. With
Turnstone's `graphshell_endpoint` binary:

```powershell
cargo run --bin a1_enrichment_receipt -- `
  ../turnstone/target/debug/graphshell_endpoint.exe `
  receipts/a1-turnstone.html `
  receipts/a1-turnstone-correlation.json
```

## A2

A2 permits sealed external evidence to qualify a reading. Cleromancy copies the
portable-card fields actually inspected by correlation, binds each card and the
source set with digests, and stores them in a v2 reading receipt. Each distinct
correlated term declared in a candidate's tags adds one base-weight share. The
receipt discloses the terms, additions, final weights, and selection.

Replay recomputes the correlation and qualification after the external endpoint
has closed. The evidence digest detects receipt changes but is not a source
signature or trust claim.

```powershell
cargo run --bin a2_enriched_receipt -- `
  ../turnstone/target/debug/graphshell_endpoint.exe `
  receipts/a2-turnstone.html `
  receipts/a2-turnstone-reading.json
```

## A3

A3 exposes three context-card commands through Graphshell: deterministic
`read`, securely random `select`, and uniformly weighted `roll`. The containing
host binds a Servitor subject outside the payload, and Servitor gates each verb
at its own scope. Accepted commands append replayable reading cards and announce
the new projection revision.

The proof carrier is in-process but still round-trips the Graphshell JSON wire.
It trusts its containing host for identity. Graphshell stdio does not
authenticate peers and must remain read-only; a remote command service needs an
authenticated carrier and session admission.

```powershell
cargo test --test a3
cargo run --bin a3_intent_receipt -- `
  receipts/a3-intent.html `
  receipts/a3-intent.json
```

The full boundary and result contract are in
`design_docs/2026-08-03_a3_bound_intents.md`.

## A4

A4 maps selected Cleromancy contexts and readings onto Graphshell H7's signed
personal-graph operations. Reading sync also carries each exact candidate
field required for replay. Sync is compiled with the `personal-sync` feature
and remains off until the user selects contexts or contexts with readings.
Graphshell retains identity, roster admission, causal storage, conflict
detection, and transport ownership.

Context facts can be sensitive. H7 protects admitted writers and network
transport, but A4 does not claim that its retained operation store is encrypted
at rest. Concurrent values for a selected Cleromancy facet are refused, and
deletions are not imported in this slice.

```powershell
cargo test --features personal-sync --test a4
cargo run --features personal-sync --bin a4_sync_receipt -- `
  receipts/a4-sync.html `
  receipts/a4-sync.json
```

The proof exchanges one signed operation between independent in-memory H7
replicas. It proves the product adapter and rematerialization path, not a fresh
resident LogSync or physical-network run. See
`design_docs/2026-08-03_a4_personal_sync_adapter.md`.

## A5

A5 makes the candidate field first-class graph truth. Every accepted reading
now points to both its context and a digest-addressed `cleromancy.field/v1`
node containing the exact candidates, rules, weights, tags, and authored
interpretations used. Equal fields deduplicate; `CleromancyHost::replay_reading`
resolves both dependencies from the graph, so an A3 caller or catalog does not
need to remain installed.

Graphshell projects fields as visible cards. Full A4 reading sync includes
them, refuses unresolved field conflicts, and rejects a reading whose field is
absent before changing local truth. Context-only sync does not carry fields.

```powershell
cargo test --test a5
cargo run --bin a5_field_receipt -- `
  receipts/a5-field.html `
  receipts/a5-field.json
```

See `design_docs/2026-08-03_a5_field_provenance.md` for the compatibility and
privacy boundary.

## A6

A6 supplies the first real consumer of A5 field nodes: a bounded, text-only
Major Arcana pack in Rider-Waite-Smith order, with Strength VIII and Justice
XI. Traditional card titles are paired with original upright reflective
prompts. The pack declares neither reversals nor astrology correspondences.

The user chooses the qualification openly. `Uniform` gives all 22 cards one
share and requires a secure cast. `Contextual` adds one base-weight share for
each matching context tag, then permits either a deterministic maximum or a
secure weighted cast. The selected rule, qualified weights, selection method,
and exact pack-derived field remain visible in the receipt and graph.

```powershell
cargo test --test a6
cargo run --bin a6_tarot_receipt -- `
  receipts/a6-tarot.html `
  receipts/a6-tarot.json
```

See `design_docs/2026-08-03_a6_major_arcana_pack.md` for the content and rule
boundary.

## A7

A7 separates the sealed result from the saved occasion. A
`cleromancy.reading-session/v1` node records local time, a CSPRNG event nonce,
the exact context and field, ordered result placements, and an optional opaque
caller token. Repeating a deterministic read can therefore save two distinct
sessions while both point to the same replayable result. A separately addressed
immutable reflection can elaborate a session without changing it or the result.

Graphshell `read`, `select`, and `roll` commands now create a session. Accepted
still means resnapshot: a caller supplies a non-secret token, waits for the
revision notice, and finds the matching session card. `ContextsAndReadings`
syncs sessions with their replay dependencies. Reflections need the explicit
`ContextsReadingsAndReflections` selection.

```powershell
cargo test --test a7
cargo test --features personal-sync --test a7_sync
cargo run --bin a7_session_receipt -- `
  receipts/a7-session.html `
  receipts/a7-session.json
```

This is the data and projection trunk, not a headed reading editor or a
multi-card spread system. See
`design_docs/2026-08-04_a7_reading_sessions.md` for its sync and privacy
boundary.

## A8

A8 adds one authored three-card layout without changing the A7 session schema.
Each secure cast is saved at `foundation`, `tension`, or `next_step`, and a
`cleromancy.three-card-spread/v1` node commits those bindings plus two explicit
graph relationships: tension tests the foundation, and the next step answers
the tension. The spread card, session, sealed results, context, and field all
remain separately inspectable and replayable.

`ContextsAndReadings` sync now carries spread nodes as selected graph truth;
the H7 adapter remains the owner of identity, admission, causal history,
conflicts, storage, and transport.

```powershell
cargo test --test a8
cargo test --features personal-sync --test a8_sync
cargo run --bin a8_three_card_receipt -- `
  receipts/a8-three-card.html `
  receipts/a8-three-card.json
```

See `design_docs/2026-08-04_a8_three_card_spread.md` for the fixed layout,
sync boundary, and stop rule.

## A9

A9 exposes the fixed spread through the authenticated Graphshell intent seam.
Context cards now advertise `cleromancy.three-card-spread` with a bounded field
payload and optional client token. The containing host must bind a subject and
the dedicated Servitor write scope. Accepted calls append the three secure
casts, session, and authored spread, then emit the ordinary revision notice.

This proves the wire contract without turning Cleromancy's generic receipt
view into a pretend editor. The headed form and input ownership remain a Mere
host concern.

```powershell
cargo test --test a3
cargo test --test a9
```

See `design_docs/2026-08-04_a9_three_card_intent.md` for the contract and
stop rule.

## License

MIT OR Apache-2.0.
