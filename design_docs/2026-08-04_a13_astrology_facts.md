# Cleromancy A13: astrology calculation facts

**Date:** 2026-08-04  
**Scope:** preserve an ephemeris result and derive replayable structured facts.

## Contract

`AstrologyChart` is a source-qualified receipt for positions supplied by an
ephemeris adapter. It retains the adapter algorithm, engine, ephemeris, UTC
instant, optional coordinates, and every body position. Longitudes and
latitudes use integer millidegrees; locations use integer microdegrees so the
digest is stable without floating-point serialization.

`AstrologyChart::facts` derives zodiac sign placement and explicitly orb-bound
major aspects. `AstrologyFacts` binds those derived values to the chart digest
and can verify them after the chart source is no longer available.

This slice does not calculate ephemerides, parse timestamps, infer houses, or
generate interpretations. A caller must name the calculation source and bring
its own adapter output. The derived facts are suitable disclosed context for a
field composer or another reading system.

## Acceptance

1. Source metadata and exact positions survive serde and affect the chart
   digest.
2. Sign and aspect derivation is integer-only, deterministic, and replayable.
3. Invalid coordinates, duplicate bodies, invalid positions, and oversized
   aspect orbs fail before a facts receipt is accepted.
4. No astrology prose, predictive claim, house system, or tarot correspondence
   is introduced.

## Stop rule

Do not add an ephemeris dependency, chart UI, or interpretation catalog in this
slice. The next boundary should be an adapter and graph projection decision,
not an implicit astrology engine.
