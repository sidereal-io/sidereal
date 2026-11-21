<!-- Plain language: Make each record link identify the decision so a reader can find its rationale. Keep this manifest separate from the records themselves. -->

# ADR Review Manifest

- Status: completed
- Review date: 2026-11-20

## Review Summary

ADR review completed for this change. The one record that governs it is ADR-016, which already decides to retire the Nix flake (D3). This change carries out that decision and makes no new architectural choice.

## In-Force ADRs Reviewed

- [ADR-016: Development Environment with mise](../../../docs/decisions/ADR-016-development-environment-with-mise.md) — decides that mise and `mise.lock` pin the tools, that the flake is retired, and that the plain pin files stay. Its consequence "Two routes exist until the flake is removed" ends with this change.
- [ADR-001: Plugin boundary](../../../docs/decisions/ADR-001-plugin-boundary.md), [ADR-002: Core and domain pack split](../../../docs/decisions/ADR-002-core-domain-pack-split.md), [ADR-003: Asset identity and content revisions](../../../docs/decisions/ADR-003-asset-identity-and-content-revisions.md), [ADR-004: Database engine and schema](../../../docs/decisions/ADR-004-database-engine-and-schema.md), [ADR-005: Visual design system](../../../docs/decisions/ADR-005-visual-design-system.md), [ADR-006: Rule engine deferral](../../../docs/decisions/ADR-006-rule-engine-deferral.md), [ADR-007: Security and plugin trust](../../../docs/decisions/ADR-007-security-and-plugin-trust.md), [ADR-008: Facet schema and write authority](../../../docs/decisions/ADR-008-facet-schema-and-write-authority.md), [ADR-009: Backend language](../../../docs/decisions/ADR-009-backend-language.md), [ADR-010: Migration strategy](../../../docs/decisions/ADR-010-migration-strategy.md), [ADR-011: Storage tree layout](../../../docs/decisions/ADR-011-storage-tree-layout.md), [ADR-014: Managed storage identity](../../../docs/decisions/ADR-014-managed-storage-identity.md), [ADR-015: Opaque database sessions](../../../docs/decisions/ADR-015-opaque-database-sessions.md) — read for context. None covers the development environment, and this change touches none of their decisions.

ADR-013 (the Nix flake) is superseded by ADR-016 and keeps its text as history (D2). ADR-012 is Proposed, not in force.

## New Durable ADRs Created

- None - no major durable architectural decisions were introduced. Each choice in design.md, such as the section names, where the move note goes and how the spec deltas are shaped, can be changed later from the code and docs alone, at little cost.
