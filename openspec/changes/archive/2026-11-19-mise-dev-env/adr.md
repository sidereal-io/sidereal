# ADR Review Manifest

- Status: completed
- Review date: 2026-11-19

## Review Summary

ADR review completed for this change. One decision qualifies: design.md Decision 6, choosing mise in place of the Nix flake. It reverses ADR-013, and undoing it later would mean rebuilding the flake, the CI jobs and the docs. The new record supersedes ADR-013, whose status now says so.

The other decisions do not qualify:

- Pinning the flake input to the Let's Spec tag (Decision 1) lasts only until the flake is removed, and is one line in `flake.nix`.
- The contents of `mise.toml` (Decision 2), committing the npm sidecar (Decision 3), the ignored personal files (Decision 4), and installing with `--locked` (Decision 5) each live in one file and are cheap to change.
- The docs changes (Decision 7) follow from the others.
- Rust following rustup's stable channel on the mise route is a consequence of the new record, and the record says so.

## In-Force ADRs Reviewed

- [ADR-001](../../../docs/decisions/ADR-001-plugin-boundary.md): Plugin Contract and Execution Profiles. No code changes.
- [ADR-002](../../../docs/decisions/ADR-002-core-domain-pack-split.md): Core / Domain-Pack Seam. No code changes.
- [ADR-003](../../../docs/decisions/ADR-003-asset-identity-and-content-revisions.md): Asset Identity and Content Revisions
- [ADR-004](../../../docs/decisions/ADR-004-database-engine-and-schema.md): Database Engine and Schema Strategy. The PostgreSQL fixture keeps running through Docker.
- [ADR-005](../../../docs/decisions/ADR-005-visual-design-system.md): v2 Visual Design System
- [ADR-006](../../../docs/decisions/ADR-006-rule-engine-deferral.md): Declarative Processing, Selectors, and Policy Deferral
- [ADR-007](../../../docs/decisions/ADR-007-security-and-plugin-trust.md): Security and Plugin Trust Model. It covers plugins, not development tools.
- [ADR-008](../../../docs/decisions/ADR-008-facet-schema-and-write-authority.md): Metadata Envelope, Facets, and Write Authority
- [ADR-009](../../../docs/decisions/ADR-009-backend-language.md): Backend Language and Runtime. Rust stays on the stable channel.
- [ADR-010](../../../docs/decisions/ADR-010-migration-strategy.md): Migration Strategy. The `v0.x` branch keeps its own toolchain.
- [ADR-011](../../../docs/decisions/ADR-011-storage-tree-layout.md): Storage Tree Layout and Cross-Filesystem Moves
- [ADR-013](../../../docs/decisions/ADR-013-development-environment.md): Development Environment. Superseded by ADR-016.
- [ADR-014](../../../docs/decisions/ADR-014-managed-storage-identity.md): Managed Storage Identity
- [ADR-015](../../../docs/decisions/ADR-015-opaque-database-sessions.md): Opaque Database-Backed Sessions

ADR-012 (Embedded Scripting Engine) is Proposed, not in force, and does not bear on this change.

## New Durable ADRs Created

- [ADR-016](../../../docs/decisions/ADR-016-development-environment-with-mise.md): Development Environment with mise. Supersedes ADR-013.
