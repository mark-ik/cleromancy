# Cleromancy A20: resident session authority

**Date:** 2026-08-07
**Scope:** share Cleromancy's durable reading graph across concurrent admitted
Graphshell endpoints without sharing connection-local projection state.

## Contract

`CleromancySessionAuthority` owns one `CleromancyApp`: its reading graph,
Servitor grants and audit, and configured intent limits. Its catalog
registration is a factory. Every `AdmittedEndpointContext` opens a new
`CleromancySessionEndpoint` with its own projection session, disclosed
resources, active scene instances, last rendered revision, pending notice, and
bound Servitor subject.

An endpoint temporarily enters its local state while it reads or writes the
shared authority. The authority serializes that transition, so Cleromancy has
one graph mutation path and no copied graph state to reconcile. A resource or
action target disclosed to one session is therefore unavailable to another,
while an accepted write changes the one shared graph.

## Revision bell

The endpoint's last rendered revision is distinct from the authority's current
graph revision. After one admitted session writes, another endpoint that has
already mounted receives one `CarrierNotice` naming its own session. Its next
snapshot sees the durable graph change. Repeated polls do not repeat the same
bell.

The existing explicit `CleromancyHost::persist` behavior remains unchanged.
A20 shares live graph truth between resident sessions; it does not add an
automatic persistence policy or peer transport.

## Acceptance

```powershell
cargo test --features graphshell-admission --test a20_resident_session_authority --offline
```

The receipt opens a writer and reader from the same catalog registration under
different admitted sessions. The writer alone has the Servitor write grant and
submits the bounded concurrence action. The reader receives one notice under
its own session, then snapshots the resulting Pattern occasion card.

## Ownership

Graphshell still owns admission, endpoint selection, carrier lifetime, and
continued authority checks. Cleromancy owns durable reading truth, local
projection state, Servitor authorization, and interpretation-specific intents.

## Stop rule

This is an in-process resident authority. It does not decide browser route
selection, shared persistence, peer replication, or generic endpoint state
management for other products.
