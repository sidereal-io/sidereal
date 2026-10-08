# ADR Review Manifest

- Status: completed
- Review date: 2026-10-08

## Review Summary

ADR review completed for this change. The highest existing sequence number is 013.
No ADR declares a supersession relationship.
The design implements the accepted database and crate-boundary decisions without choosing a new engine or architecture.
Dedicated database ownership follows this story's requirement. PostgreSQL version support, bootstrap details, and probe deadlines remain change-level design choices.
The user confirmed that this database story has no authentication prerequisite.
Auth planning owns ADR-007's broad sequencing wording; this change preserves the limited health-only HTTP surface.

## In-Force ADRs Reviewed

- [ADR-001: Plugin contract](../../../docs/decisions/ADR-001-plugin-boundary.md)
- [ADR-002: Core and domain packs](../../../docs/decisions/ADR-002-core-domain-pack-split.md)
- [ADR-003: Asset identity](../../../docs/decisions/ADR-003-asset-identity-and-content-revisions.md)
- [ADR-004: Database engine](../../../docs/decisions/ADR-004-database-engine-and-schema.md)
- [ADR-005: Visual design](../../../docs/decisions/ADR-005-visual-design-system.md)
- [ADR-006: Processing goals](../../../docs/decisions/ADR-006-rule-engine-deferral.md)
- [ADR-007: Security](../../../docs/decisions/ADR-007-security-and-plugin-trust.md)
- [ADR-008: Facets](../../../docs/decisions/ADR-008-facet-schema-and-write-authority.md)
- [ADR-009: Backend language](../../../docs/decisions/ADR-009-backend-language.md)
- [ADR-010: Migration strategy](../../../docs/decisions/ADR-010-migration-strategy.md)
- [ADR-011: Storage layout](../../../docs/decisions/ADR-011-storage-tree-layout.md)
- [ADR-013: Development environment](../../../docs/decisions/ADR-013-development-environment.md)

ADR-012 remains Proposed and is outside this change's scripting-free scope.

## New Durable ADRs Created

None. No major durable architectural decisions were introduced and no new repository-level ADR files were created.
