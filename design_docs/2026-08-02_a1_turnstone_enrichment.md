# Cleromancy A1: source-owned enrichment

**Date:** 2026-08-02
**Scope:** external Graphshell projection mounting and transparent correlation.

## Boundary

Turnstone already owns a Graphshell endpoint over its live Mere graph.
Cleromancy reaches it through `graphshell_protocol::Carrier`, using discovery,
snapshot, and resource requests. It does not depend on Turnstone's Rust types,
open Turnstone storage, or import its nodes and facets.

The external session mounts in the same `graphshell_client::ClientState` as the
local Cleromancy session. Each endpoint retains its own session, resources,
cache policy, semantics, and source truth.

## Correlation

`cleromancy.enrichment/lexical-overlap/v1` is deliberately small and visible:

1. lowercase alphanumeric tokens of at least three characters are extracted
   from the context label, tags, fact names, and fact values;
2. the same tokenizer reads disclosed portable-card semantics, titles, badges,
   labels, and values;
3. each card reports the exact intersection, sorted.

The report names the context digest, endpoint, projection, and external
session. It is presentation-side evidence only. It does not change weights,
become a Mere facet, or enter a reading receipt in A1.

## Acceptance

1. Local and external sessions are mounted concurrently in one Graphshell
   client.
2. External resources are fetched only after their session advertises them.
3. Correlation is deterministic and discloses every query and matched term.
4. Mounting and correlation leave the Cleromancy graph byte-identical.
5. A cross-process receipt mounts Turnstone's real `graphshell_endpoint` and
   resolves its three source-owned cards.

## Stop rule

Stop before external evidence affects Lachesis, before remote intents, and
before Turnstone or Cleromancy personal sync. The next reading-affecting slice
must extend the receipt so replay can name or carry the exact external evidence
that changed qualification.
