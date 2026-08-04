# Cleromancy A12: field composer draft

**Date:** 2026-08-04  
**Scope:** provide a typed, serializable authoring draft for generic candidate
fields.

## Contract

`FieldComposer` carries the system identifier, declared rule, and ordered
`Candidate` values under `cleromancy.field-composer/v1`. A caller can add
candidates incrementally, serialize the draft for a local UI, and call
`finish` to emit the ordinary `Field` used by the reading engine and v2
composition payload.

The draft checks only structural facts the composer can know locally:

- system and rules are nonempty;
- at least one candidate exists;
- candidate IDs are nonempty and unique; and
- base weights are nonzero.

It does not invent titles or interpretations, choose a rule, or pretend that a
rule is executable. Rule-specific checks remain in Lachesis and the reading
receipt. A uniform field still needs unit weights when it is actually read.

## Ownership boundary

Cleromancy owns this generic field authoring model. Mere can bind controls to
the draft, list graph-resident fields by digest, and submit the resulting
inline or stored composition through its existing typed invocation path.

## Acceptance

1. A valid draft emits a byte-equivalent `Field` and survives serde round-trip.
2. Duplicate IDs, zero weights, empty systems, and empty drafts fail before an
   intent is constructed.
3. Existing A10/A11 composition and graph selection paths remain unchanged.

## Stop rule

Do not add generated interpretations, astrology semantics, or a generic JSON
editor here. Those remain separate product and content decisions.
