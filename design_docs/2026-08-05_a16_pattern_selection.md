# Cleromancy A16: saved pattern selection

**Date:** 2026-08-05
**Scope:** let an authorized Graphshell host save an A15 pattern occasion by
selecting existing astrology facts and an existing reading session.

## Contract

`cleromancy.create-concurrence` carries
`cleromancy.intent.create-concurrence/v1` with an astrology-facts digest and a
reading-session ID. The action is advertised on both matching saved cards, not
on a context card. Its target must be one of the exact submitted members.

Before asking Servitor for the dedicated write scope, Cleromancy resolves the
facts from their canonical address, verifies them against their stored chart,
and replays the selected reading session. A bad selection is rejected without
a graph mutation or revision notice. A successful call records the ordinary
A15 collection-membership graph truth and emits the normal resnapshot notice.

## Ownership boundary

Mere owns the headed chooser that presents compatible cards and constructs the
typed payload. Cleromancy owns member identity, replay validation,
authorization dispatch, and the saved occasion. The portable cards are a
selection index, not an interpretation interface.

## Acceptance

1. Saved astrology-facts and reading-session cards advertise the action.
2. The selected target must match either member named by the payload.
3. An authorized action saves one replayable concurrence and emits a revision
   notice.
4. A mismatched target or absent member is rejected before graph mutation.
5. The projected result still states the non-causal concurrence claim.

## Stop rule

Do not build a raw JSON form, infer a correspondence, alter Tarot weights, or
treat the action as a headed chooser. A Mere surface may add that chooser by
selecting compatible cards and sending this payload. Personal sync remains
unchanged until astrology facts have explicit opt-in selection.
