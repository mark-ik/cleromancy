# Cleromancy A11: graph-resident field selection

**Date:** 2026-08-04  
**Scope:** let the generic composition action select an existing field node by
digest without copying its candidate payload into a headed form.

## Contract

`cleromancy.intent.compose-reading/v2` keeps the A10 composition fields and
changes `field` to a tagged selection:

- `{ "kind": "inline", "field": ... }` carries a newly composed field;
- `{ "kind": "stored", "digest": "..." }` names an existing graph-resident
  field.

The stored form is useful for a Mere composer. A field card now discloses its
canonical digest, so a host can let the user choose a known field and send the
digest rather than reconstructing candidate interpretations from a summary
card.

## Resolution boundary

Cleromancy resolves a stored digest against
`cleromancy://field/{digest}`, verifies the facet's canonical digest, and only
then petitions Servitor or casts a reading. Missing fields are rejected before
graph mutation or a revision notice. Inline fields retain the A10 behavior and
remain bounded by the existing candidate and payload limits.

## Ownership boundary

Mere owns field-node selection and the headed composer controls. Cleromancy
owns field identity, exact candidate truth, resolution, calculation, and
retention. The card summary is a selection affordance, not an interpretation
source.

## Acceptance

1. A stored-field composition accepts through the authenticated wire and saves
   a session bound to the selected digest.
2. A missing stored field is rejected without Servitor mutation or a notice.
3. Inline A10 compositions and the narrow A3/A9 actions remain accepted.

## Stop rule

Do not expose raw field facet JSON as a generic Graphshell form. The next UI
slice can list disclosed field cards and emit this typed digest selection; new
candidate authoring remains a Cleromancy-owned editor decision.
