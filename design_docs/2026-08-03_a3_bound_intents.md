# Cleromancy A3: host-bound reading intents

**Date:** 2026-08-03
**Scope:** expose deterministic readings, secure selections, and dice through
Graphshell without letting a payload invent its caller.

## Boundary

Graphshell `IntentInvocation` carries a session, target, observed revision,
intent name, and payload. It does not carry a proved caller identity. The
containing transport host must therefore bind a Servitor `Subject` out of band
before Cleromancy advertises or accepts commands. Payload schemas contain no
identity field.

The in-process `LocalCarrier` is A3's second-consumer proof. It serializes every
request and response through the Graphshell JSON wire format, but it trusts the
containing host for the bound subject. Graphshell stdio explicitly provides no
authentication, so Cleromancy commands must not be exposed over stdio. A remote
command service requires a carrier that authenticates its peer and binds that
identity to the session.

The direct `CleromancyHost` endpoint remains read-only. The composed
`CleromancyApp` endpoint advertises actions on context cards only while a
subject is bound.

## Commands

- `cleromancy.read` applies the declared qualification deterministically.
- `cleromancy.select` casts with operating-system entropy across the same
  qualified field.
- `cleromancy.roll` builds a uniformly weighted `dN` field and casts once.

Read and select accept an optional sealed A2 enrichment snapshot. Every payload
names a versioned schema. Configurable limits bound encoded payload bytes,
candidate count, and die sides before execution.

Each verb petitions Servitor for write access to its own scope:

- `cleromancy/intents/read`
- `cleromancy/intents/select`
- `cleromancy/intents/roll`

A grant on the parent `cleromancy/intents` scope may deliberately cover all
three. Authorization is recorded in Servitor's attributed audit graph. The
resulting reading and replay receipt are recorded in Cleromancy's local graph.
These are separate records with separate ownership.

## Result contract

`IntentResult::Accepted` is an acknowledgement, not a result body. Acceptance
appends a reading card and emits a `CarrierNotice` with the new projection
revision. The consumer snapshots again to observe the result. An invocation
against an old revision receives `IntentResult::Stale` and performs no work.

Malformed, oversized, unadvertised, unbound, or unauthorized requests receive
`IntentResult::Rejected`. Invalid field or sealed-evidence data is also a
rejection. Failure of the operating-system entropy source is an endpoint
failure because no valid cleromantic result exists to append.

## Acceptance

1. LocalCarrier round-trips discovery, snapshot, and all three commands through
   JSON encoding.
2. Only a bound app projection advertises the three versioned actions.
3. A parent grant permits read, select, and roll; an exact read grant does not
   permit select.
4. Unbound and unauthorized commands append nothing and emit no notice.
5. Accepted commands append replayable readings, emit revision notices, and
   make the old invocation stale.
6. A deterministic read receipt is byte-stable across fresh proof runs.

## Stop rule

Stop before an authenticated network carrier, session admission, p2p sync,
multi-die notation, tarot and astrology catalogs, interpretation generation,
and a generic plug-in SDK. A3 does not claim that LocalCarrier or stdio proves
a remote peer identity.
