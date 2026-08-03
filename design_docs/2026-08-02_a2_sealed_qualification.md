# Cleromancy A2: sealed external qualification

**Date:** 2026-08-02
**Scope:** let disclosed external evidence affect a reading without making the
external endpoint part of replay.

## Boundary

Before qualification, Cleromancy copies the exact portable-card strings read
by `cleromancy.enrichment/lexical-overlap/v1`: the presentation label, card
title, badges, and labeled values. Each card receives a content digest and the
ordered source set receives an evidence digest. The receipt carries that
snapshot. Turnstone's graph, scene, resources, and storage remain source-owned.

The digest binds the receipt to disclosed bytes. It is not a signature and does
not establish that the endpoint or its statements were trustworthy.

## Qualification

`cleromancy.qualification/external-term-share/v1` is explicit:

1. verify every card digest, the evidence digest, algorithm, and context digest;
2. recompute the lexical correlation report from the sealed cards and context;
3. form the set of distinct correlated terms across all cards;
4. intersect that set with each candidate's declared tags;
5. add one candidate base-weight share for each distinct matching term.

Repeated cards cannot multiply the same term. Context qualification remains the
baseline. Concurrence with sealed external evidence adds a separately disclosed
share. Calculated readings take the new maximum; cast readings draw across the
same externally qualified field.

As of A6, this behavior is selected only by the field rule
`contextual-weight+external-term-share/v1`. Using that rule without its sealed
evidence is an error rather than an implicit contextual reading.

## Receipt

An enriched receipt is `cleromancy.receipt/v2`. It carries:

- the complete sealed evidence snapshot and evidence digest;
- the correlation report digest;
- matched terms for every candidate;
- exact per-candidate weight additions;
- final weights, selection mode, bounded sample when cast, and selected result.

Replay requires the original context and field, as in A0, but does not contact
the external endpoint. Any change to a card field, derivation, weight, or
selection fails replay.

## Acceptance

1. The fixture evidence changes the calculated winner and both calculated and
   cast readings replay after endpoint shutdown.
2. Tampered sealed card data is rejected.
3. Mounting and sealing leave the local Mere graph byte-identical.
4. The local Graphshell card discloses source, evidence digest, matches, and
   additions.
5. A real Turnstone stdio receipt is byte-stable across fresh endpoint runs.

## Stop rule

Stop before remote intents, personal sync, source signatures or trust scores,
tarot and astrology catalogs, and generic plug-in APIs. No external evidence
may influence a reading through a rule other than the versioned qualifier named
in its receipt.
