# ADR Review Manifest

- Status: completed
- Review date: 2026-11-19

## Review Summary

ADR review completed for this change. Every in-force record was read. The one that governs this change is [ADR-016](../../../docs/decisions/ADR-016-development-environment-with-mise.md), which already decides that CI installs its tools from the same `mise.lock` as contributors. This change carries out that decision for the `ci` workflow and creates no record.

## In-Force ADRs Reviewed

- [ADR-001: Plugin Contract and Execution Profiles](../../../docs/decisions/ADR-001-plugin-boundary.md)
- [ADR-002: Core / Domain-Pack Seam](../../../docs/decisions/ADR-002-core-domain-pack-split.md)
- [ADR-003: Asset Identity and Content Revisions](../../../docs/decisions/ADR-003-asset-identity-and-content-revisions.md)
- [ADR-004: Database Engine and Schema Strategy](../../../docs/decisions/ADR-004-database-engine-and-schema.md)
- [ADR-005: v2 Visual Design System](../../../docs/decisions/ADR-005-visual-design-system.md)
- [ADR-006: Declarative Processing, Selectors, and Policy Deferral](../../../docs/decisions/ADR-006-rule-engine-deferral.md)
- [ADR-007: Security and Plugin Trust Model](../../../docs/decisions/ADR-007-security-and-plugin-trust.md)
- [ADR-008: Metadata Envelope, Facets, and Write Authority](../../../docs/decisions/ADR-008-facet-schema-and-write-authority.md)
- [ADR-009: Backend Language and Runtime](../../../docs/decisions/ADR-009-backend-language.md)
- [ADR-010: Migration Strategy — Clean Break with a One-Way Importer](../../../docs/decisions/ADR-010-migration-strategy.md)
- [ADR-011: Storage Tree Layout and Cross-Filesystem Moves](../../../docs/decisions/ADR-011-storage-tree-layout.md)
- [ADR-014: Managed Storage Identity](../../../docs/decisions/ADR-014-managed-storage-identity.md)
- [ADR-015: Opaque Database-Backed Sessions](../../../docs/decisions/ADR-015-opaque-database-sessions.md)
- [ADR-016: Development Environment with mise](../../../docs/decisions/ADR-016-development-environment-with-mise.md)

Not in force, so read for context only: ADR-012 (Proposed) and ADR-013 (superseded by ADR-016).

## New Durable ADRs Created

- None - no major durable architectural decisions were introduced. Each of design.md's decisions is cheap to reverse in one edit to `.github/workflows/ci.yml`: one install step in every job (D-1), mise-action's cache left on (D-2), cargo caching through `mise exec` (D-3), the `compose` job running its recipe (D-4), the `e2e` and `lets-spec` steps kept (D-5), and the spec delta's shape (D-6). The doc corrections (D-7) are not a fork. The costly choice, mise instead of Nix for CI, is ADR-016's.
