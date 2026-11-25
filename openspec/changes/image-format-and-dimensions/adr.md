<!-- Plain language: Make each record link identify the decision so a reader can find its rationale. Keep this manifest separate from the records themselves. -->

# ADR Review Manifest

- Status: completed
- Review date: 2026-11-25

## Review Summary

ADR review completed for this change. None of design.md's decisions needs a
new record. Decision 1 (the format comes from the media type, not a facet)
follows from ADR-008: one pack owns a facet's schema, so a format facet owned
by `packs/image` could not take formats from other packs. It adds no
contract, type or table, and can be changed inside one story. Decisions 2 to
5 (the read's `media_type`, the header reader, the upload's Operator list and
log lines, and the asset page) can also be changed inside one story.

The first build of this change wrote ADR-018 for a choice value type. The
user removed the choice type from this change after the pull request review,
and ADR-018 with it. It never reached `main`, so it is deleted, not
superseded, and no other document links to it.

## In-Force ADRs Reviewed

- [ADR-001: Plugin Contract and Execution Profiles](../../../docs/decisions/ADR-001-plugin-boundary.md)
- [ADR-002: Core / Domain-Pack Seam](../../../docs/decisions/ADR-002-core-domain-pack-split.md) — the header Operator lives in `packs/image`, which depends on `plugin-abi` only.
- [ADR-003: Asset Identity and Content Revisions](../../../docs/decisions/ADR-003-asset-identity-and-content-revisions.md)
- [ADR-004: Database Engine and Schema Strategy](../../../docs/decisions/ADR-004-database-engine-and-schema.md) — this change adds no migration.
- [ADR-005: v2 Visual Design System](../../../docs/decisions/ADR-005-visual-design-system.md) — the asset page adds no token.
- [ADR-006: Declarative Processing, Selectors, and Policy Deferral](../../../docs/decisions/ADR-006-rule-engine-deferral.md)
- [ADR-007: Security and Plugin Trust Model](../../../docs/decisions/ADR-007-security-and-plugin-trust.md)
- [ADR-008: Metadata Envelope, Facets, and Write Authority](../../../docs/decisions/ADR-008-facet-schema-and-write-authority.md) — one pack owns a facet's schema, which is why the format is not a facet (design decision 1).
- [ADR-009: Backend Language and Runtime](../../../docs/decisions/ADR-009-backend-language.md)
- [ADR-010: Migration Strategy — Clean Break with a One-Way Importer](../../../docs/decisions/ADR-010-migration-strategy.md)
- [ADR-011: Storage Tree Layout and Cross-Filesystem Moves](../../../docs/decisions/ADR-011-storage-tree-layout.md)
- [ADR-014: Managed Storage Identity](../../../docs/decisions/ADR-014-managed-storage-identity.md)
- [ADR-015: Opaque Database-Backed Sessions](../../../docs/decisions/ADR-015-opaque-database-sessions.md)
- [ADR-016: Development Environment with mise](../../../docs/decisions/ADR-016-development-environment-with-mise.md)
- [ADR-017: Facet Values as Rows with a JSONB Value](../../../docs/decisions/ADR-017-facet-value-rows.md) — still in force; this change adds no value type and no migration.

ADR-012 (proposed) and ADR-013 (superseded by ADR-016) were read but are not
in force.

## New Durable ADRs Created

- None - no major durable architectural decisions were introduced.
