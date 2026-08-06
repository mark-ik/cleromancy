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

## A10

A10 adds `cleromancy.compose-reading`, a single typed action for a headed host
to choose an explicit field, a `single` or `three_card` layout, and
`calculated` or `cast` mode. A single composition uses the existing reading
engine; a three-card cast uses the authored A8 spread. The exact field and
client-correlated session remain graph truth, while impossible combinations are
rejected before mutation.

The older narrow actions remain available. Mere owns the future field editor:
this contract gives it a stable payload and resnapshot path without turning the
generic receipt renderer into an editor.

```powershell
cargo test --test a10
```

See `design_docs/2026-08-04_a10_generic_composer.md` for the dispatch table and
ownership boundary.

## A11

A11 lets the generic composer select an existing graph-resident field by its
canonical digest. `cleromancy.intent.compose-reading/v2` accepts either an
inline field or a tagged stored-field reference. Cleromancy resolves and
verifies the stored facet before authorization and mutation; a missing field is
rejected without a notice. Field cards disclose the digest so Mere can build a
selection control without copying candidate interpretations into a generic
form.

```powershell
cargo test --test a10
```

See `design_docs/2026-08-04_a11_field_selection.md` for the resolution and
ownership boundary.

## A12

A12 adds `FieldComposer`, a serializable draft for authoring generic candidate
fields. It validates only local structure, then emits the same exact `Field`
used by the reading engine and composition intent. It does not generate
interpretations or decide whether a declared rule is executable.

```powershell
cargo test --test a12
```

See `design_docs/2026-08-04_a12_field_composer.md` for the validation and
ownership boundary.

## A13

A13 adds an explicit astrology calculation boundary. `AstrologyChart` records
the named ephemeris adapter, engine, ephemeris, UTC instant, optional
coordinates, and integer body positions. `AstrologyFacts` deterministically
derives zodiac placements and orb-bounded major aspects from those positions,
then binds the result back to the chart digest.

This is structured context, not an ephemeris engine or an interpretation
catalog. It does not parse timestamps, infer houses, generate prose, or claim
prediction. The source calculation remains inspectable and replayable before a
caller combines the facts with a field or reading.

```powershell
cargo test --test a13
```

See `design_docs/2026-08-04_a13_astrology_facts.md` for the source and
interpretation boundary.

## A14

A14 defines `AstrologyAdapter`, which receives an explicit moment and returns a
source-qualified chart. The host now stores the chart and its derived facts as
separate digest-addressed graph nodes, links facts to their chart, and exposes
both through ordinary portable cards. Replay recomputes facts from stored
positions and the declared orb without needing the original adapter.

The concrete ephemeris, personal-sync selection, chart UI, houses, and
interpretation catalog remain open decisions.

```powershell
cargo test --test a14
```

See `design_docs/2026-08-04_a14_astrology_adapter_graph.md` for the adapter
and graph ownership boundary.

## A15

A15 saves one cross-system pattern occasion. A `Concurrence` node groups an
exact astrology facts node with an exact reading session using collection
membership. It records that the two were consulted together while explicitly
declining to claim that the astrology caused, qualified, or explained the
Tarot cast.

The executable proof uses disclosed fixture positions and fixed test entropy,
then writes an inspectable Graphshell graph and JSON receipt. Production casts
still use operating-system entropy, and no astrology correspondence is applied
to Tarot weights.

```powershell
cargo test --test a15
cargo run --bin a15_pattern_receipt -- `
  receipts/a15-pattern.html `
  receipts/a15-pattern.json
```

See `design_docs/2026-08-05_a15_pattern_occasion.md` for the concurrence claim
and stop rule.

## A16

A16 makes the A15 occasion selectable through the bound Graphshell action
seam. Saved astrology-facts and reading-session cards advertise
`cleromancy.create-concurrence` only when both replayed saved-value sets exist.
The advertised form supplies the exact astrology-facts and reading-session
values the endpoint accepts, with no host default or free-text ID entry. Its
typed payload names both exact members, and the action target must itself be
one of them. Cleromancy replays both saved values before seeking the dedicated
Servitor write scope and adding the occasion; an accepted call emits the normal
revision notice.

The action is the contract a headed Mere chooser needs. The chooser itself is
still a host surface, and the resulting card retains the limited “consulted
together” claim.

```powershell
cargo test --test a16
cargo run --bin a16_pattern_selection_receipt -- `
  receipts/a16-pattern-selection.html `
  receipts/a16-pattern-selection.json
```

See `design_docs/2026-08-05_a16_pattern_selection.md` for the target-binding
and ownership boundary.

## A17

A17 proves the same bounded action through Graphshell's carrier-neutral
retained session. Cleromancy is mounted through its real wire-round-tripping
`LocalCarrier`; Graphshell opens the advertised form from the semantic tree,
submits exact endpoint choices, then resnapshots and finds the saved Pattern
occasion card. This is an integration proof, not a new Cleromancy form or a
browser transport.

```powershell
cargo test --test a17_graphshell_action_draft --offline
```

See `design_docs/2026-08-06_a17_retained_graphshell_action_draft.md` for the
carrier and ownership boundary.

## A18

A18 gives Cleromancy an opt-in adapter for a Graphshell session that the
containing host has already admitted. `AdmittedEndpointContext` supplies the
transcript-derived projection session and public-key subject. Cleromancy binds
its in-memory endpoint to that session and maps the subject to Servitor before
it advertises a write action. The reading graph and persistent local storage
remain Cleromancy's own.

The context is an in-process handoff, not a portable authentication token. The
surrounding Graphshell session loop still owns admission, expiry, revocation,
and browser transport. A resident endpoint catalog and actual browser routing
remain the next composition gate.

```powershell
cargo test --features graphshell-admission --test a18_admitted_endpoint --offline
```

See `design_docs/2026-08-06_a18_admitted_endpoint.md` for the handoff and
stop rule.

## License

MIT OR Apache-2.0.
