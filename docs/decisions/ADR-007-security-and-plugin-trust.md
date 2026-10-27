---
id: adrs-adr007
date: 2026-08-18
amended: 2026-10-27
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

**Authentication and access.** An unclaimed instance has no users. The first visitor claims it by creating the one administrator with a username and password, or the operator does the same from the box. The claim is the one unauthenticated mutation, and it is permitted only while no administrator exists. Every other mutation needs an authenticated session. Browser access uses secure, HTTP-only, same-site sessions, with a CSRF token on state-changing requests. CORS is deny-by-default with exact-match origins, never a credentialed wildcard. WebSocket auth derives from the session and rechecks expiry and revocation. A read-only public gallery, if added, is a separate surface that must be explicitly enabled and has no reach into the admin API.

**Plugin trust.** Built-in Rust is trusted first-party code. Script bundles and external providers are untrusted by default. Installation displays and records their requested capabilities: byte access, facet writes, network destinations, secret names, and core-managed mutations. Grant changes require admin confirmation and invalidate active plugin sessions. Plugins have no ambient access to the environment, processes, network, database, or assets.

**External providers and secrets.** External providers authenticate per-instance over TLS/mTLS or a protected local transport, with identity pinned. A callback carrying a run id is rejected on any run, provider, or grant mismatch. Secrets are encrypted at rest, never returned through read APIs after creation, scoped per run, and redacted from logs, errors, progress, and history. Trust-relevant events are audited.

**Prerequisites before implementation.** Each prerequisite gates the surface it protects, not the whole programme. The auth mode and setup flow gate the claim and session routes, and are now chosen. Specified and integration-tested HTTP, WebSocket, CORS and CSRF behaviour gates each protected surface as it is built. The grant vocabulary and approval flow gate plugin installation. External-provider auth and secret redaction, with testable fixtures, gate the first provider integration. The threat model for a malicious script, a compromised provider and an unauthenticated network caller gates the surface each threat reaches.

## Consequences

- File mutation and plugin management sit behind an enforceable boundary from day one, with an auditable trail of installs, grants, secret access, and mutations.
- Multi-user roles, per-collection sharing, and reverse-proxy trusted-header auth are deferred, but explicitly additive later.
- "Encrypted at rest" defends only a narrow threat until the key custody model is pinned down. On a single-box Docker deploy, if the key lives on the same volume as the ciphertext, it barely protects against the most likely attacker (volume access). If the key is derived from the admin password, losing that password loses every secret. What the encryption is meant to defend against, and where key material lives, is decided by the first feature that stores a provider secret, in its own record.

## Amendment, 2026-10-27

The setup flow is chosen: a first-visitor claim, as Immich does it, instead of a credential minted or imported at setup. The exposure is the window between the first start and the first visit, which the server narrows by warning in its log on every start until the instance is claimed, and which the operator closes by claiming from the box. The prerequisites now gate per surface, so the claim and session routes could be built before the plugin grant vocabulary existed. Key custody moved out of the setup flow to the first feature that needs it.

## Alternatives Considered

### Alternative 1: Preserve the trusted-network model (no built-in auth)
- **Pros:** simplest; operators supply a reverse proxy and network isolation.
- **Cons:** hands any network caller file-mutation and plugin-management authority; leaves secure WebSocket and browser behaviour deployment-specific.
- **Why not:** a system of record that mutates irreplaceable files cannot delegate its own access control to whatever network it lands on.

### Alternative 2: Full multi-user RBAC in v2.0
- **Pros:** the broadest model — administrators, editors, viewers, plugin managers, and per-collection grants.
- **Cons:** funds an Immich-like sharing surface the architecture explicitly defers.
- **Why not:** it builds a multi-user product before the single-user one exists. Single-user auth is the enforceable boundary that RBAC can extend later.
