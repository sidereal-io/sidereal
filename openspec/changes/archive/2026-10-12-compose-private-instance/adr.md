# ADR Review Manifest

- Status: completed
- Review date: 2026-10-12

## Review Summary

ADR review completed for this change.

The design's choices are packaging details that a later change can reverse cheaply:

- **Where the web files come from** (a folder, not the binary). Changing it touches one setting and one build stage.
- **How Compose passes the password.** Changing it touches `compose.yml` and the guide.
- **Where verification runs.** A CI job can call the same recipe later.

Docker Compose as the installation path, PostgreSQL as the only engine, and the pre-authentication boundary are all decided elsewhere. This change follows them.

## In-Force ADRs Reviewed

- `docs/decisions/ADR-001-plugin-boundary.md` — accepted. Not affected.
- `docs/decisions/ADR-002-core-domain-pack-split.md` — accepted. Static serving stays in the `server` crate, so `core` gains no HTTP code.
- `docs/decisions/ADR-003-asset-identity-and-content-revisions.md` — accepted. Not affected.
- `docs/decisions/ADR-004-database-engine-and-schema.md` — accepted. The instance runs PostgreSQL only.
- `docs/decisions/ADR-005-visual-design-system.md` — accepted. The change adds no token and no screen.
- `docs/decisions/ADR-006-rule-engine-deferral.md` — accepted. Not affected.
- `docs/decisions/ADR-007-security-and-plugin-trust.md` — accepted. See the exception below.
- `docs/decisions/ADR-008-facet-schema-and-write-authority.md` — accepted. Not affected.
- `docs/decisions/ADR-009-backend-language.md` — accepted. The server stays in Rust.
- `docs/decisions/ADR-010-migration-strategy.md` — accepted. The change adds nothing from v0.10.x.
- `docs/decisions/ADR-011-storage-tree-layout.md` — accepted. The change mounts a storage volume and writes nothing to it.
- `docs/decisions/ADR-013-development-environment.md` — accepted. The Nix shell is unchanged. Docker still runs outside it.

`docs/decisions/ADR-012-embedded-scripting-engine.md` is proposed, not in force. This change does not depend on it.

No ADR supersedes another.

### ADR-007 exception

ADR-007 says implementation cannot begin until its security prerequisites are met. Those prerequisites are not met yet.
This change proceeds under a recorded exception, not under the ADR's own text:

- **What the exception allows.** Story #389's packet permits packaging that exposes only setup and health behavior. This change serves static files and two health probes. It adds no mutation and no data access.
- **Who owns the wording.** The authentication epic, #388, owns ADR-007's wording and the access controls. This change does not edit ADR-007.
- **The precedent.** The database foundation, #394, recorded the same boundary before it merged.

## New Durable ADRs Created

- None - no major durable architectural decisions were introduced.
