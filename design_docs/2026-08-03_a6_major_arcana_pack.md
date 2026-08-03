# Cleromancy A6: explicit Major Arcana qualification

**Date:** 2026-08-03
**Scope:** make one bounded Tarot pack the first authored consumer of A5 field
nodes and expose the material difference between a uniform cast and contextual
qualification.

## Contradiction

Before A6, a field retained a `rules` string, but Lachesis always applied the
same contextual tag-weighting algorithm. The receipt therefore described the
actual arithmetic while the stored rule remained decorative. That becomes
untenable once a catalog offers a user-visible choice between an unqualified
draw and a context-shaped reading.

A6 makes the recognized rule identifiers executable contracts. Unknown rules
are rejected. A rule that requires external evidence cannot run without its
sealed evidence. A uniform field must contain unit base weights and cannot be
used with calculated maximum selection.

## Pack

`cleromancy.tarot/rws-major-reflective-v1` contains the 22 Major Arcana in
Rider-Waite-Smith order, with Strength VIII and Justice XI. It uses traditional
card titles and original Cleromancy reflective prompts. Each card has exactly
three plain context tags and a base weight of one.

This is a built-in data pack, not a catalog-installation or plug-in API. It is
text-only and upright-only. It makes no claims about reversals, images,
astrological correspondences, spreads, or generated narrative.

## User-visible qualifications

The pack can create two distinct fields:

- `Uniform` uses `uniform/v1`. Context is ignored, every card receives one
  share, and selection must be a cast. Production uses operating-system
  cryptographic entropy with unbiased bounded selection.
- `Contextual` uses `contextual-weight/v1`. Every matching context tag adds one
  base-weight share. A reading may take the disclosed maximum deterministically
  or cast securely across those disclosed weights.

The choice is not an interpretation style. It changes the calculation, field
digest, receipt algorithm, and graph node. A UI should therefore present it as
a setting before selection, using concrete labels such as “Uniform cast” and
“Context-weighted reading.”

The A6 proof uses fixed entropy solely to produce a byte-stable uniform receipt.
The receipt states that fact. The public `ReadingEngine::cast` path continues
to use operating-system entropy.

## Graph truth

Both pack variants become ordinary A5 `cleromancy.field/v1` nodes. Each reading
points to its exact field and context. Inserting the uniform and contextual
proofs therefore yields one shared context, two distinct fields, two readings,
and four `GeneratedFrom` relations. Both readings replay using only the graph.

The serialized pack and its digest are proof metadata. Replay authority remains
the retained field node, so the pack does not introduce a parallel source of
truth.

## Acceptance

1. The built-in pack contains 22 unique candidates in the declared order.
2. A uniform cast gives every candidate weight one and records the uniform
   cast algorithm.
3. A contextual calculation discloses its tag-derived weights and may select a
   different candidate from the same ordered pack.
4. The two qualification choices produce distinct field nodes and both
   readings replay from graph-resident context and field data.
5. Unknown rules, non-unit uniform candidates, calculated uniform selection,
   and evidence-dependent rules without evidence are rejected.
6. The HTML and JSON proof receipts are byte-stable.

## Limits and stop rule

A6 proves a one-card suggestion mechanism, not a complete Tarot practice or a
prediction engine. Stop before multi-card spreads, card positions, reversals,
images, astrology correspondences, pack installation, generated interpretation,
or generic plug-in APIs. A later correspondence slice must represent its own
versioned facts rather than hiding them inside contextual tags.
