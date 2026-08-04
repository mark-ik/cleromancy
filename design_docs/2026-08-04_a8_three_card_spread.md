# Cleromancy A8: one authored three-card spread

**Date:** 2026-08-04
**Scope:** make one useful multi-card reading frame concrete without turning
Cleromancy into a spread language.

## Model

A8 keeps A7's `cleromancy.reading-session/v1` record unchanged. The session
already preserves an ordered list of result placements, so a three-card cast
uses the positions `foundation`, `tension`, and `next_step`. Each card is an
independent secure cast against the same disclosed context and field. The
session's event nonce and optional client token remain separate from the card
receipts.

`cleromancy.three-card-spread/v1` is an immutable node attached to that
session. It commits the three position-to-reading bindings and exactly two
authored relationships:

- `tension` questions `foundation`, labelled `tests the foundation`;
- `next_step` follows `tension`, labelled `answers the tension`.

These are product-authored reading structure, not claims derived from card
meaning. The spread's graph edges use Mere's `Questions` and `NextStep`
semantic kinds, so the orrery can show the authored relationships alongside
the cards, session, context, and field.

## Tarot as the first consumer

The proof uses the A6 RWS major-arcana pack with an explicitly uniform field.
The cards retain their sealed candidate, calculation, bounded sample, and
event nonce. A8 does not add reversals, astrology correspondences, generated
prose, or a generic pack/spread plug-in API.

## Sync

The selected `ContextsAndReadings` lane now includes spread facets alongside
contexts, fields, readings, and sessions. The adapter raises its batch schema
to v4. H7 still owns identity, admission, causal history, conflict handling,
storage, and transport. Import validates every spread, its session, each
position binding, and every sealed reading before mutating local graph truth.

## Acceptance

1. One API call produces three independently sealed readings in the fixed
   positions and one saved session.
2. The spread identity rejects changed positions, card references, or authored
   relationship labels.
3. Graph projection contains the spread node, session/result containment, and
   the two semantic relationship edges.
4. Spread replay resolves the session and all three readings from graph-resident
   context, field, and receipt truth.
5. A signed two-replica H7 exchange round-trips the spread and preserves the
   exported event digest.

## Limits and next boundary

This is one authored layout, not a configurable spread DSL. The next host
surface is the Mere headed editor that can start a named reading and display
saved history. Astrology and other correspondence systems should enter as
separate field/context consumers, not as hidden spread interpretation.
