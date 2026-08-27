---
id: adrs-adr007
date: 2026-08-18
status: accepted
title: 'ADR007: Security and Plugin Trust Model'
description: Architecture Decision Record (ADR) for the v2 trust boundary — built-in single-user authentication and explicit, per-install plugin capability grants.
---

# ADR-007: Security and Plugin Trust Model

## Context

v2 becomes the system of record for user files and can invoke code that reads assets, publishes externally, and requests secrets, so the current unauthenticated/CORS-open posture cannot be carried forward implicitly. Before any code touches a real filesystem the architecture must define who may use the HTTP and WebSocket APIs, which actions need elevated authority, whether a plugin is trusted and which capabilities it gets, how external providers authenticate, how secrets are stored/delivered/logged/revoked, and which origins may issue authenticated requests. Treating every self-hosted network as trusted is insufficient — reverse proxies, shared home networks, browser CSRF, and accidentally exposed container ports are ordinary conditions. This is the trust half of the plugin model in [ADR-001 — Plugin contract & execution profiles](ADR-001-plugin-boundary.md).

## Decision

Adopt **built-in single-user authentication and explicit plugin grants**. Initial setup mints or imports one administrator credential; there is no unauthenticated mutation mode. Browser access uses secure, HTTP-only, same-site sessions with a CSRF token on state-changing requests; CORS is deny-by-default with exact-match origins (never a credentialed wildcard); WebSocket auth derives from the session and rechecks expiry/revocation; a read-only public gallery, if added, is a separate explicitly-enabled surface with no admin-API reach. Built-in Rust is trusted first-party code; script bundles and external providers are untrusted by default, and installation displays and records their requested capabilities (byte access, facet writes, network destinations, secret names, core-managed mutations). Grant changes require admin confirmation and invalidate active plugin sessions; plugins have no ambient environment/process/network/database/asset access. External providers authenticate per-instance over TLS/mTLS or a protected local transport with identity pinned, and callbacks carrying a run id are rejected on any run/provider/grant mismatch. Secrets are encrypted at rest, never returned through read APIs after creation, scoped per run, and redacted from logs/errors/progress/history; trust-relevant events are audited. Implementation cannot begin until the auth mode and setup flow are chosen; HTTP/WebSocket/CORS/CSRF behaviour is specified and integration-tested; the grant vocabulary and approval flow are defined; external-provider auth and secret redaction have testable fixtures; and the threat model covers a malicious script, a compromised provider, and an unauthenticated network caller.

## Consequences

- File mutation and plugin management are behind an enforceable boundary from day one, with an auditable trail of installs, grants, secret access, and mutations.
- Multi-user roles, per-collection sharing, and reverse-proxy trusted-header auth are deferred but explicitly additive later.
- "Encrypted at rest" defends a narrow threat until the key custody model is pinned down: for a single-box Docker deploy, if the key lives on the same volume as the ciphertext it barely protects against the most likely attacker (volume access), and if it is derived from the admin password, losing that password loses every secret. What the encryption is meant to defend against, and where key material lives, must be stated with the setup flow.

## Alternatives Considered

### Alternative 1: Preserve the trusted-network model (no built-in auth)
- **Pros:** simplest; operators supply a reverse proxy and network isolation.
- **Cons:** hands any network caller file-mutation and plugin-management authority; leaves secure WebSocket and browser behaviour deployment-specific.
- **Why not:** a system of record that mutates irreplaceable files cannot delegate its own access control to whatever network it lands on.

### Alternative 2: Full multi-user RBAC in v2.0
- **Pros:** the broadest model — administrators, editors, viewers, plugin managers, per-collection grants.
- **Cons:** funds an Immich-like sharing surface the architecture explicitly defers.
- **Why not:** it builds a multi-user product before the single-user one exists; single-user auth is the enforceable boundary that RBAC can extend later.
