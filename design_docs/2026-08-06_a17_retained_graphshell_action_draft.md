# Cleromancy A17: retained Graphshell action draft

**Scope:** prove that the A16 bounded concurrence action can be driven through
Graphshell's reusable retained-session host, rather than a Cleromancy-local
payload helper.

## Contract

The A17 test creates one saved astrology facts record and one saved reading
session, binds the Servitor subject, and mounts `CleromancyApp` through its
wire-round-tripping `LocalCarrier`. Graphshell discovers and mounts that live
endpoint through `RetainedEndpointSession::over`.

From the mounted accessibility tree, Graphshell opens the advertised
`cleromancy.create-concurrence` form on the saved facts card. The draft exposes
only the endpoint's exact facts digest and session ID. Submission without a
selection stays local and fails before a carrier request. Submission with both
advertised values is accepted by Cleromancy; Graphshell then requests a fresh
snapshot and observes the new Pattern occasion card.

## Ownership

Graphshell owns the carrier-neutral session, draft state, acknowledgement
capture, and resnapshot mechanics. Cleromancy owns the action descriptor,
member replay, Servitor write grant, and `Concurrence` persistence. The test
does not add a Cleromancy form renderer or a browser transport.

## Acceptance

```powershell
cargo test --test a17_graphshell_action_draft --offline
```

The test proves exact choices, local missing-field rejection, endpoint
acceptance, a later observed revision, and visible projected concurrence.

## Stop rule

Do not make a generic host select domain members by heuristic, use free-text
identifiers, or represent this in browser code before an admitted browser
carrier can mount Cleromancy safely.
