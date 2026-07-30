# 007: Security and Plugin Trust Model

**Status:** Proposed
**Date:** 2026-07-30
**Context:** M0 of [RFC #213](https://github.com/sidereal-io/sidereal/issues/213). v2 adds filesystem mutation, plugin execution, external providers, and scoped secret delivery. The current unauthenticated/CORS-open deployment posture cannot be carried forward implicitly.

## Problem

Sidereal v2 becomes the system of record for user files and can invoke code that reads assets,
publishes externally, and requests secrets. Before M1 touches a real filesystem, the architecture
must define:

- who may use the HTTP and WebSocket APIs;
- which actions require elevated authority;
- whether a plugin is trusted and which capabilities it receives;
- how external-provider endpoints are authenticated;
- how secrets are stored, delivered, logged, and revoked;
- which browser origins can issue authenticated requests.

Treating every self-hosted network as trusted is not sufficient. Reverse proxies, shared home
networks, browser-based CSRF, and accidentally exposed container ports are ordinary deployment
conditions.

## Options

### Option A: Preserve the trusted-network model

No built-in authentication; operators are responsible for a reverse proxy and network isolation.

This is simple, but it gives any network caller file-mutation and plugin-management authority and
leaves secure WebSocket and browser behavior deployment-specific.

### Option B: Built-in single-user authentication and explicit plugin grants

Sidereal owns one administrative identity initially, authenticated browser sessions, CSRF protection,
restricted origins, and per-install plugin capability approval. Multi-user roles can be added later.

This fits the single-user product while establishing an enforceable trust boundary.

### Option C: Full multi-user RBAC in v2.0

Model administrators, editors, viewers, plugin managers, and per-collection grants before cutover.

This is the broadest design but funds an Immich-like sharing surface the architecture explicitly
defers.

## Recommendation

Choose **Option B** for v2.0.

### Application boundary

- Initial setup creates or imports one administrator credential; there is no unauthenticated
  mutation mode.
- Browser access uses secure, HTTP-only, same-site sessions. State-changing HTTP requests require a
  CSRF token.
- CORS is deny-by-default. Configured origins are exact matches; credentials are never combined with
  a wildcard origin.
- WebSocket authentication derives from the same session and rechecks expiry/revocation.
- Read-only public gallery access, if added, is a separate explicitly enabled surface with no admin
  API reachability.
- Reverse-proxy authentication may be supported later as an explicit trusted-header mode with a
  configured proxy allowlist, never by accepting arbitrary identity headers.

### Plugin trust and capabilities

- Built-in Rust code is trusted first-party code and is covered by the release threat model.
- Script bundles and external providers are untrusted by default. Installation displays and records
  requested capabilities: byte access, facet writes, network destinations, secret names, and
  core-managed mutations.
- Grant changes require administrator confirmation and invalidate active plugin sessions.
- Plugins have no ambient environment, process, network, database, or asset-store access.
- External providers authenticate with a plugin-instance credential over TLS, mutually authenticated
  TLS, or a protected local transport. Provider identity is pinned to its configured instance.
- External-provider callbacks carry the run ID and are rejected if the run, provider, or grant does
  not match.

### Secrets and audit

- Secrets are encrypted at rest using deployment-managed key material and are never returned through
  normal read APIs after creation.
- A run receives only declared secrets, scoped to the plugin instance. Secrets are redacted from
  logs, errors, progress payloads, and operation history.
- Plugin install, grant, revoke, secret access, provider connection, file mutation, and publish
  events are auditable.
- Backup and migration documentation states how encrypted secrets and key material are handled.

## M0 exit criteria

M1 cannot begin until:

1. the application authentication mode and initial-setup flow are selected;
2. HTTP, WebSocket, CORS, and CSRF behavior are specified and covered by integration tests;
3. the manifest grant vocabulary and installation-approval flow are defined;
4. external-provider authentication and secret redaction have testable protocol fixtures;
5. the threat model covers a malicious script, a compromised provider, and an unauthenticated network
   caller.

## Decision

[Filled in after review.]
