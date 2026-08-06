# Cleromancy A18: admitted endpoint adapter

**Date:** 2026-08-06
**Scope:** bind Cleromancy's endpoint to a Graphshell session already admitted
by its composing host.

## Contract

Graphshell's native `SessionAuthority` can derive an
`AdmittedEndpointContext`. The context contains only two facts:

- the transcript-derived `ProjectionSession` that the endpoint must serve;
- the admitted public-key subject as a 32-byte key.

It contains no delegation, transport, expiry record, or revocation ledger. It
is an in-process composition handoff, not a portable proof of admission.
`SessionAuthority` and the admitted Graphshell loop continue to recheck those
things before any endpoint request is dispatched.

With the opt-in `graphshell-admission` feature, `CleromancyApp` implements
Graphshell's `BindAdmittedSession` contract. Binding changes the ephemeral
projection session, clears resources and active presentation bindings from any
earlier session, and maps the admitted public key to the existing Servitor
`Subject`. Durable graph truth remains local and is not keyed to or persisted
with that session name.

## Acceptance

```powershell
cargo test --features graphshell-admission --test a18_admitted_endpoint --offline
```

The A18 proof creates a typed admitted context, binds a populated Cleromancy
app through that contract, and mounts it with Graphshell's real retained
carrier session. The mounted projection has the admitted session identifier.
The bound Servitor subject can save an A16 pattern occasion using only the
advertised facts digest and reading-session ID. A fresh snapshot then shows
the saved Pattern occasion.

Graphshell's lifecycle test separately proves that a real
`SessionAuthority` derives the same context values from its admitted session
and principal.

## Ownership

Graphshell owns the carrier admission, continued expiry/revocation checks, and
the context handoff. Cleromancy owns its local graph, endpoint projection,
choice validation, and Servitor petition. The public-key shape is shared by
design, but a Cleromancy adapter does not open a Personae vault or issue a new
delegation.

## Stop rule

This is not a live browser route. Do not make `graphshell_device_host` depend
on Cleromancy, add a second native host that owns the same vault, serialize the
context into an IPC bearer token, or claim browser end-to-end proof. The next
gate is a resident endpoint catalog whose host can select a product endpoint
and keep its admitted session loop around it.
