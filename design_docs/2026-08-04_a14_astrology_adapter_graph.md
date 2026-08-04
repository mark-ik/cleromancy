# Cleromancy A14: astrology adapter and graph projection

**Date:** 2026-08-04  
**Scope:** define the ephemeris adapter seam and retain its output in the
local Mere graph.

## Contract

`AstrologyAdapter` is the only calculation boundary. An adapter receives an
explicit `AstrologyMoment` and returns an `AstrologyChart` carrying its own
algorithm, engine, ephemeris, and exact positions. `calculate_with_adapter`
rejects a receipt for a different moment before it can become graph truth.

`CleromancyHost::insert_astrology_chart` stores two separate nodes:

- `cleromancy://astrology/chart/{chart_digest}` with
  `cleromancy.astrology-chart/v1`;
- `cleromancy://astrology/facts/{facts_digest}` with
  `cleromancy.astrology-facts/v1`.

The facts node has a provenance edge to the chart node. Replay recomputes the
structured facts from the stored chart and declared orb. Graphshell cards show
source metadata, positions, chart digest, placements, and aspects.

## Ownership boundary

The adapter owns ephemeris mathematics and source licensing. Cleromancy owns
receipt validation, integer normalization, sign/aspect derivation, graph
identity, and projection. This slice does not choose a concrete ephemeris,
publish astrology nodes through personal sync, calculate houses, or generate
interpretations.

## Acceptance

1. An adapter cannot return a chart for another requested moment.
2. Chart and facts nodes are addressable, linked, replayable, and visible as
   portable cards.
3. Missing chart dependencies fail before replay and do not become fabricated
   facts.

## Stop rule

Choose the concrete ephemeris and its licensing/runtime boundary before adding
personal-sync facets or chart UI. Keep the source receipt and derived facts
separately selectable.
