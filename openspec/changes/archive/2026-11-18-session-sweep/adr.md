# ADR Review Manifest

- Status: completed
- Review date: 2026-11-18

## Review Summary

ADR review completed for this change. No decision qualifies for a record.

- The delete as a `Database` method in `core`, and the task beside `relay` in the server crate, follow the split the code already has. Moving either is a small refactor that no other code depends on.
- The in-process task instead of a job runner is a stopgap the story names as such. A later job runner replaces one spawned task and one call in `main.rs`, so the choice is cheap to reverse.
- The `Schedule` value, the `interval_at` loop, writing through `Log`, and the line wording are local to the sweep and its tests.

## In-Force ADRs Reviewed

- [ADR-001](../../../docs/decisions/ADR-001-plugin-boundary.md): Plugin Contract and Execution Profiles
- [ADR-002](../../../docs/decisions/ADR-002-core-domain-pack-split.md): Core / Domain-Pack Seam. Sessions are access, not astronomy, so the delete belongs in `core`.
- [ADR-003](../../../docs/decisions/ADR-003-asset-identity-and-content-revisions.md): Asset Identity and Content Revisions
- [ADR-004](../../../docs/decisions/ADR-004-database-engine-and-schema.md): Database Engine and Schema Strategy. The change adds no schema.
- [ADR-005](../../../docs/decisions/ADR-005-visual-design-system.md): v2 Visual Design System
- [ADR-006](../../../docs/decisions/ADR-006-rule-engine-deferral.md): Declarative Processing, Selectors, and Policy Deferral. Its periodic sweeps reconcile processing goals; this session sweep is unrelated and does not anticipate that execution model.
- [ADR-007](../../../docs/decisions/ADR-007-security-and-plugin-trust.md): Security and Plugin Trust Model. The sweep deletes only expired sessions, which the access model already refuses.
- [ADR-008](../../../docs/decisions/ADR-008-facet-schema-and-write-authority.md): Metadata Envelope, Facets, and Write Authority
- [ADR-009](../../../docs/decisions/ADR-009-backend-language.md): Backend Language and Runtime
- [ADR-010](../../../docs/decisions/ADR-010-migration-strategy.md): Migration Strategy
- [ADR-011](../../../docs/decisions/ADR-011-storage-tree-layout.md): Storage Tree Layout and Cross-Filesystem Moves
- [ADR-013](../../../docs/decisions/ADR-013-development-environment.md): Development Environment
- [ADR-014](../../../docs/decisions/ADR-014-managed-storage-identity.md): Managed Storage Identity
- [ADR-015](../../../docs/decisions/ADR-015-opaque-database-sessions.md): Opaque Database-Backed Sessions. A session ends when its row is deleted or expired; the sweep deletes rows that are already expired.

ADR-012 (Embedded Scripting Engine) is Proposed, not in force, and does not bear on this change.

## New Durable ADRs Created

- None - no major durable architectural decisions were introduced.
