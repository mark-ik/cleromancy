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

## License

MIT OR Apache-2.0.
