---
id: adrs-adr007
date: 2026-08-18
status: accepted
title: 'ADR007: Security and Plugin Trust Model'
description: Architecture Decision Record (ADR) for the v2 trust boundary — built-in single-user authentication and explicit, per-install plugin capability grants.
---

# ADR-007: Security and Plugin Trust Model

## Context

As the primary system of record for user files, Sidereal executes code that accesses assets, broadcasts data externally, and retrieves sensitive secrets. Consequently, the existing unauthenticated and open-CORS posture is no longer viable and cannot be maintained.

Prior to any code interacting with an active filesystem, the core architecture must clearly establish:

- API access permissions for HTTP and WebSocket endpoints.
- Which operations demand elevated privileges.
- Criteria for plugin trust and their corresponding capability allocations.
- Authentication protocols for external integrations.
- Lifecycles for secret management, including storage, transport, logging, and revocation.
- Approved origins permitted to send authenticated requests.

Relying on implicit trust within self-hosted environments is insufficient, as reverse proxies, shared local networks, cross-site request forgery (CSRF), and unintended container exposure represent standard operating risks.

These requirements form the trust framework complementing the plugin specifications in
n [ADR-001 — Plugin contract & execution profiles](ADR-001-plugin-boundary.md).

## Decision

Adopt **built-in single-user authentication and explicit plugin grants.**

**Authentication and access.** Initial setup mints or imports one administrator credential. There is no unauthenticated mutation mode. Browser access uses secure, HTTP-only, same-site sessions, with a CSRF token on state-changing requests. CORS is deny-by-default with exact-match origins, never a credentialed wildcard. WebSocket auth derives from the session and rechecks expiry and revocation. A read-only public gallery, if added, is a separate surface that must be explicitly enabled and has no reach into the admin API.

**Plugin trust.** Built-in Rust is trusted first-party code. Script bundles and external providers are untrusted by default. Installation displays and records their requested capabilities: byte access, facet writes, network destinations, secret names, and core-managed mutations. Grant changes require admin confirmation and invalidate active plugin sessions. Plugins have no ambient access to the environment, processes, network, database, or assets.

**External providers and secrets.** External providers authenticate per-instance over TLS/mTLS or a protected local transport, with identity pinned. A callback carrying a run id is rejected on any run, provider, or grant mismatch. Secrets are encrypted at rest, never returned through read APIs after creation, scoped per run, and redacted from logs, errors, progress, and history. Trust-relevant events are audited.

**Prerequisites before implementation.** Implementation cannot begin until: the auth mode and setup flow are chosen; HTTP, WebSocket, CORS, and CSRF behaviour is specified and integration-tested; the grant vocabulary and approval flow are defined; external-provider auth and secret redaction have testable fixtures; and the threat model covers a malicious script, a compromised provider, and an unauthenticated network caller.

## Consequences

- File mutation and plugin management sit behind an enforceable boundary from day one, with an auditable trail of installs, grants, secret access, and mutations.
- Multi-user roles, per-collection sharing, and reverse-proxy trusted-header auth are deferred, but explicitly additive later.
- "Encrypted at rest" defends only a narrow threat until the key custody model is pinned down. On a single-box Docker deploy, if the key lives on the same volume as the ciphertext, it barely protects against the most likely attacker (volume access). If the key is derived from the admin password, losing that password loses every secret. What the encryption is meant to defend against, and where key material lives, must be stated with the setup flow.

## Alternatives Considered

### Alternative 1: Preserve the trusted-network model (no built-in auth)
- **Pros:** simplest; operators supply a reverse proxy and network isolation.
- **Cons:** hands any network caller file-mutation and plugin-management authority; leaves secure WebSocket and browser behaviour deployment-specific.
- **Why not:** a system of record that mutates irreplaceable files cannot delegate its own access control to whatever network it lands on.

### Alternative 2: Full multi-user RBAC in v2.0
- **Pros:** the broadest model — administrators, editors, viewers, plugin managers, and per-collection grants.
- **Cons:** funds an Immich-like sharing surface the architecture explicitly defers.
- **Why not:** it builds a multi-user product before the single-user one exists. Single-user auth is the enforceable boundary that RBAC can extend later.
