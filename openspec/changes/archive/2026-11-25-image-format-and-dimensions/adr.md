<!-- Plain language: Make each record link identify the decision so a reader can find its rationale. Keep this manifest separate from the records themselves. -->

# ADR Review Manifest

- Status: completed
- Review date: 2026-11-25

## Review Summary

ADR review completed for this change. Of design.md's decisions, decision 3
(core keeps a choice's allowed values with the schema version), together
with decision 1 (the choice type in the public contract), is a costly fork
that the code alone would not explain: it adds a column to the kept schema
rows, which ADR-017 expected new types to avoid, and it makes a new allowed
value a new schema version. It gets a new record, which builds on ADR-017.
Decisions 2 and 4 carry out ADR-008's rule that core checks every value
against its schema. Decisions 5 to 8 (the header reader, the facet JSON,
the upload's Operator list and log lines, and the asset page) can be
changed inside one story and need no record.

## In-Force ADRs Reviewed

- [ADR-001: Plugin Contract and Execution Profiles](../../../docs/decisions/ADR-001-plugin-boundary.md)
- [ADR-002: Core / Domain-Pack Seam](../../../docs/decisions/ADR-002-core-domain-pack-split.md) — the header Operator lives in `packs/image`, which depends on `plugin-abi` only.
- [ADR-003: Asset Identity and Content Revisions](../../../docs/decisions/ADR-003-asset-identity-and-content-revisions.md)
- [ADR-004: Database Engine and Schema Strategy](../../../docs/decisions/ADR-004-database-engine-and-schema.md) — the new migration is forward-only.
- [ADR-005: v2 Visual Design System](../../../docs/decisions/ADR-005-visual-design-system.md) — the asset page adds no token.
- [ADR-006: Declarative Processing, Selectors, and Policy Deferral](../../../docs/decisions/ADR-006-rule-engine-deferral.md)
- [ADR-007: Security and Plugin Trust Model](../../../docs/decisions/ADR-007-security-and-plugin-trust.md)
- [ADR-008: Metadata Envelope, Facets, and Write Authority](../../../docs/decisions/ADR-008-facet-schema-and-write-authority.md)
- [ADR-009: Backend Language and Runtime](../../../docs/decisions/ADR-009-backend-language.md)
- [ADR-010: Migration Strategy — Clean Break with a One-Way Importer](../../../docs/decisions/ADR-010-migration-strategy.md)
- [ADR-011: Storage Tree Layout and Cross-Filesystem Moves](../../../docs/decisions/ADR-011-storage-tree-layout.md)
- [ADR-014: Managed Storage Identity](../../../docs/decisions/ADR-014-managed-storage-identity.md)
- [ADR-015: Opaque Database-Backed Sessions](../../../docs/decisions/ADR-015-opaque-database-sessions.md)
- [ADR-016: Development Environment with mise](../../../docs/decisions/ADR-016-development-environment-with-mise.md)
- [ADR-017: Facet Values as Rows with a JSONB Value](../../../docs/decisions/ADR-017-facet-value-rows.md) — still in force; ADR-018 builds on it and narrows its expectation that a new value type needs no migration.

ADR-012 (proposed) and ADR-013 (superseded by ADR-016) were read but are not
in force.

## New Durable ADRs Created

- [ADR-018: Choice Facets Keep Their Allowed Values with the Schema Version](../../../docs/decisions/ADR-018-choice-facets-keep-allowed-values.md) — design decisions 1 and 3.
