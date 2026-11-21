<!-- Plain language: Make each record link identify the decision so a reader can find its rationale. Keep this manifest separate from the records themselves. -->

# ADR Review Manifest

- Status: completed
- Review date: 2026-11-20

## Review Summary

ADR review completed for this change. The records most relevant to it are:

- [ADR-016: Development Environment with mise](../../../docs/decisions/ADR-016-development-environment-with-mise.md). It already expects a workflow that updates `mise.lock` and runs Let's Spec's sync in one step, because Dependabot has no mise support. This change builds that workflow and does not change the decision.
- [ADR-007: Security and Plugin Trust Model](../../../docs/decisions/ADR-007-security-and-plugin-trust.md) governs plugin trust in the product, not CI tokens, so it does not constrain this change.

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

ADR-012 is Proposed and ADR-013 is superseded by ADR-016, so neither is in force.

## New Durable ADRs Created

- None - no major durable architectural decisions were introduced. Each choice in design.md (the update script's steps, the pull-request action, the token's permissions, the schedule) is cheap to reverse by editing one file.
