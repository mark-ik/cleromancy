# Cleromancy A10: generic field composition

**Date:** 2026-08-04  
**Scope:** give a headed host one typed action for composing a disclosed field
with a layout and selection mode.

## Contract

`cleromancy.compose-reading` carries
`cleromancy.intent.compose-reading/v1`:

- an explicit `Field`, including its system, rule, ordered candidates, tags,
  weights, and authored interpretations;
- `single` or `three_card` layout;
- `calculated` or `cast` selection mode;
- optional sealed enrichment for a single reading; and
- an optional bounded client token for resnapshot correlation.

The containing host still supplies the target context, authenticated subject,
and Servitor grant. The field cannot replace that context binding. The app
retains the exact field through the existing field, reading, session, and
spread facets.

## Dispatch

The composition action is an orchestration facade over the existing reading
organs, not a second calculation engine:

| layout | mode | result |
| --- | --- | --- |
| `single` | `calculated` | one deterministic reading session |
| `single` | `cast` | one secure cast reading session |
| `three_card` | `cast` | the authored A8 three-card spread |

`three_card` with `calculated`, or enrichment attached to a spread, is refused
before Servitor mutation. The older `read`, `select`, `roll`, and
`three-card-spread` actions remain available for callers that want a narrower
command.

## Ownership boundary

Cleromancy owns the composition schema, validation, dispatch, and retained
truth. Mere owns the headed field editor and controls how a user assembles or
selects candidates. A future composer should start with existing field nodes or
catalog entries, then emit this payload. It must not imply that interpretation
was generated merely because a candidate was selected.

## Acceptance

1. A bound authorized caller composes a deterministic single reading and a
   three-card cast through the same action.
2. Both sessions retain their client tokens and exact fields.
3. The three-card composition replays through the graph-resident spread.
4. Impossible layout/mode combinations are rejected without a notice or graph
   mutation.
5. Existing A3 and A9 action contracts remain accepted.

## Stop rule

Do not add a generic JSON editor to the receipt renderer. The next UI slice is
Mere-owned: a product surface can select or author fields, validate them
locally, and use the typed composition payload through its existing
acknowledgement and resnapshot path.
