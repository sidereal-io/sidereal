# 007: Security and Plugin Trust Model

**Status:** Accepted
**Date:** 2026-07-30
**Context:** M0 of [RFC #213](https://github.com/sidereal-io/sidereal/issues/213). v2 adds filesystem
mutation, plugin execution, external providers, and scoped secret delivery. The current
unauthenticated/CORS-open posture cannot be carried forward implicitly.

## Problem

v2 becomes the system of record for user files and can invoke code that reads assets, publishes
externally, and requests secrets. Before M1 touches a real filesystem, the architecture must define who
may use the HTTP and WebSocket APIs, which actions need elevated authority, whether a plugin is trusted
and which capabilities it receives, how external-provider endpoints authenticate, how secrets are
stored/delivered/logged/revoked, and which browser origins may issue authenticated requests. Treating
every self-hosted network as trusted is not sufficient — reverse proxies, shared home networks,
browser-based CSRF, and accidentally exposed container ports are ordinary conditions.

## Options

### Option A: Preserve the trusted-network model

No built-in authentication; operators supply a reverse proxy and network isolation. Simple, but it
hands any network caller file-mutation and plugin-management authority and leaves secure WebSocket and
browser behavior deployment-specific.

### Option B: Built-in single-user authentication and explicit plugin grants

One administrative identity, authenticated browser sessions, CSRF protection, restricted origins, and
per-install plugin capability approval. Multi-user roles can come later. Fits the single-user product
while establishing an enforceable trust boundary.

### Option C: Full multi-user RBAC in v2.0

Administrators, editors, viewers, plugin managers, and per-collection grants before cutover. The
broadest design, but funds an Immich-like sharing surface the architecture explicitly defers.

## Recommendation

**Option B** — it fits the single-user product and establishes an enforceable boundary, where A hands
file-mutation authority to any network caller and C funds a multi-user sharing surface v2.0 does not
need.

## Decision

Accepted 2026-08-18 (M0 of RFC #213). Adopt **Option B — built-in single-user authentication and
explicit plugin grants**. Option A is rejected because file mutation and plugin management cannot be
handed to any network caller; Option C is deferred as an Immich-like sharing surface v2.0 does not fund.

**Application boundary.**

- Initial setup creates or imports one administrator credential; there is no unauthenticated mutation
  mode.
- Browser access uses secure, HTTP-only, same-site sessions; state-changing HTTP requests carry a CSRF
  token.
- CORS is deny-by-default with exact-match configured origins; credentials are never combined with a
  wildcard origin.
- WebSocket auth derives from the same session and rechecks expiry/revocation.
- A read-only public gallery, if added, is a separate explicitly-enabled surface with no admin-API
  reachability.
- Reverse-proxy trusted-header auth may come later as an explicit mode with a configured proxy
  allowlist — never by accepting arbitrary identity headers.

**Plugin trust and capabilities.**

- Built-in Rust is trusted first-party code under the release threat model. Script bundles and external
  providers are untrusted by default; installation displays and records requested capabilities (byte
  access, facet writes, network destinations, secret names, core-managed mutations).
- Grant changes require administrator confirmation and invalidate active plugin sessions.
- Plugins have no ambient environment, process, network, database, or asset-store access.
- External providers authenticate with a plugin-instance credential over TLS, mTLS, or a protected
  local transport, identity pinned to the configured instance; callbacks carry the run ID and are
  rejected on any run/provider/grant mismatch.

**Secrets and audit.**

- Secrets are encrypted at rest with deployment-managed key material and never returned through read
  APIs after creation.
- A run receives only its declared secrets, scoped to the plugin instance, redacted from logs, errors,
  progress payloads, and history.
- Plugin install, grant, revoke, secret access, provider connection, file mutation, and publish events
  are auditable. Backup and migration docs state how encrypted secrets and key material are handled.

## M0 exit criteria

Binding — M1 cannot begin until:

1. the application authentication mode and initial-setup flow are selected;
2. HTTP, WebSocket, CORS, and CSRF behavior are specified and covered by integration tests;
3. the manifest grant vocabulary and installation-approval flow are defined;
4. external-provider authentication and secret redaction have testable protocol fixtures;
5. the threat model covers a malicious script, a compromised provider, and an unauthenticated network
   caller.
