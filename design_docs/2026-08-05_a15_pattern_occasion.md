# Cleromancy A15: cross-system pattern occasion

**Date:** 2026-08-05
**Scope:** save independently produced astrology facts and a reading session as
one inspectable occasion.

## Contract

`Concurrence` is an immutable, content-addressed grouping of two or more local
graph addresses. Each member has a role, but membership means only that the
values were consulted together during the recorded occasion. The concurrence
does not assert that one member caused, qualified, explained, or interpreted
another.

`Concurrence::astrology_reading` binds an exact astrology facts digest and an
exact reading session ID. `CleromancyHost::insert_concurrence` resolves every
member before mutation, stores `cleromancy.concurrence/v1`, and adds ordinary
collection-membership edges from the pattern occasion to its members. Replay
requires the stored concurrence and all named members.

The A15 executable makes the boundary visible. It uses disclosed fixture
positions, derives astrology facts, performs a uniform Tarot cast,
saves the reading session, groups the facts and session, and writes Graphshell
HTML plus a JSON receipt. Fixed entropy makes the proof repeatable; production
casts continue to use operating-system entropy.

## Acceptance

1. Member ordering and concurrence identity are canonical and replayable.
2. Missing or duplicate members fail before graph mutation.
3. Astrology facts and the reading session remain independent graph nodes,
   connected only through the pattern-occasion collection.
4. The projected card and JSON receipt disclose the limited concurrence claim.

## Stop rule

Do not infer a correspondence, alter Tarot weights, or present concurrence as
prediction. A later correspondence pack must name its author, version, mapping,
and qualification algorithm explicitly. Concurrence is also excluded from
personal sync until its astrology members have their own opt-in selection.
