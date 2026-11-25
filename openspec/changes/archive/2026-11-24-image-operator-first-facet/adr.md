<!-- Plain language: Make each record link identify the decision so a reader can find its rationale. Keep this manifest separate from the records themselves. -->

# ADR Review Manifest

- Status: completed
- Review date: 2026-11-23

## Review Summary

ADR review completed for this change. Of design.md's decisions, only
decision 4 (how facet values are stored) is a costly fork that the code
alone would not explain. It gets a new record, which replaces one rule in
ADR-004. Decision 2's registration check carries out ADR-008's ownership
rule, and decision D4's built-in grants follow ADR-007's trust in
built-in code; neither needs a new record. Decisions 1, 5, 7, 8 and 9 can
be changed inside one story.

## In-Force ADRs Reviewed

- [ADR-001: Plugin Contract and Execution Profiles](../../../docs/decisions/ADR-001-plugin-boundary.md)
- [ADR-002: Core / Domain-Pack Seam](../../../docs/decisions/ADR-002-core-domain-pack-split.md)
- [ADR-003: Asset Identity and Content Revisions](../../../docs/decisions/ADR-003-asset-identity-and-content-revisions.md)
- [ADR-004: Database Engine and Schema Strategy](../../../docs/decisions/ADR-004-database-engine-and-schema.md) — its facet-storage rule is replaced by ADR-017; it now points there.
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

ADR-012 (proposed) and ADR-013 (superseded by ADR-016) were read but are not
in force.

## New Durable ADRs Created

- [ADR-017: Facet Values as Rows with a JSONB Value](../../../docs/decisions/ADR-017-facet-value-rows.md) — design decision 4.
