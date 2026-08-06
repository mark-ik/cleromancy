# Cleromancy A19: resident endpoint catalog

**Date:** 2026-08-06
**Scope:** open Cleromancy from Graphshell's host-owned local endpoint catalog
after admission.

## Contract

`ResidentEndpointCatalog` holds local route registrations. A registration's
factory receives the already-admitted `AdmittedEndpointContext` and returns
one endpoint for that session. The `cleromancy` factory binds that exact
context through the existing `BindAdmittedSession` adapter before Graphshell
mounts it through its retained carrier session.

The route key is host configuration. It does not come from a browser request,
does not authenticate a caller, and does not leave the native process. The
catalog holds neither a Personae vault nor a delegation chain, and it does not
replace the session loop that owns continued authority checks.

## Acceptance

```powershell
cargo test --features graphshell-admission --test a19_resident_endpoint_catalog --offline
```

The proof builds saved astrology facts and a saved reading session, opens the
`cleromancy` registration with an admitted public-key subject, and mounts that
endpoint using Graphshell's `LocalCarrier`. The catalog-bound subject submits
the bounded concurrence action. A fresh projection finds the resulting
Pattern occasion card. The A19 binary passed this proof.

## Ownership

Graphshell owns the catalog, admission context, carrier loop, and endpoint
selection. Cleromancy owns endpoint state, local graph persistence, Servitor
authorization, choice validation, and its revision notices. The catalog's
notifying registration preserves those notices without learning their product
meaning.

## Stop rule

This is not browser endpoint selection, network transport, cross-process
context passing, or a Graphshell dependency on Cleromancy. The next gate is to
choose an endpoint from a configured browser or resident-host route while
retaining the same admission and carrier authority boundary.
