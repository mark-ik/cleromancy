# Cleromancy A9: headed-host three-card intent

**Date:** 2026-08-04
**Scope:** expose the fixed A8 spread through the existing authenticated
Graphshell intent seam without pretending the current receipt view is an
editor.

## Contract

`cleromancy.three-card-spread` is advertised on context cards beside the A3
single-reading actions. Its payload is
`cleromancy.intent.three-card-spread/v1` and contains an explicit candidate
field plus an optional bounded, opaque client token. The target context comes
from the current projection instance; the payload cannot replace that binding.

The containing transport must bind a Servitor subject. Cleromancy checks the
current scene revision, payload size, candidate count, token length, and the
dedicated `cleromancy/intents/three-card-spread` write scope before calling the
A8 host writer. A successful call appends three secure casts, one session, and
one authored spread node, then emits the normal projection notice. The caller
resnapshots to inspect the resulting session and spread cards.

## Ownership boundary

The action contract is Cleromancy-owned. Mere Graphshell owns the headed
presentation, action controls, and future form for choosing or editing a
field. The current generic projection receipt renders advertised actions but
does not claim to collect input. A9 therefore proves a real wire consumer
without modifying the Mere donor port.

## Acceptance

1. Context cards advertise the three-card intent and its payload schema.
2. A bound, authorized current-revision invocation is accepted and emits a
   notice.
3. The resnapshot contains a three-placement session and spread card, and the
   spread replays from graph-resident truth.
4. Existing A3 actions remain accepted and their action set now includes the
   explicit spread action.
5. Missing subject, missing Servitor grant, stale revision, malformed payload,
   and oversize limits remain rejected before domain mutation.

## Stop rule

Do not add HTML form code to Cleromancy or teach the generic receipt to fake an
editor. The next UI slice belongs in the Mere headed host after its action
surface has a product-owned input and resnapshot path.
