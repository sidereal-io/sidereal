# ADR Review Manifest

- Status: completed
- Review date: 2026-10-29

## Review Summary

ADR review completed for this change. Every decision in design.md was tested against the `lets-write-adrs` rule: a fork among viable approaches that an engineer could plausibly reverse from the code alone, at a real cost. None passes both parts. The binary's crate, the hand-rolled argument parser, the termios read and the connect-without-migrating constructor each live in a few lines of code and change at the cost of those lines. Two behaviors must not be reversed by accident: the command never migrates, and a reset ends every session in one transaction. The `instance-access` requirements hold both, and a test asserts each. The decisions that give the command its place, claiming from the box and revocation as a row delete, are already recorded in ADR-007 and ADR-015.

## In-Force ADRs Reviewed

- [ADR-001: Plugin Contract and Execution Profiles](../../../docs/decisions/ADR-001-plugin-boundary.md), accepted
- [ADR-002: Core / Domain-Pack Seam](../../../docs/decisions/ADR-002-core-domain-pack-split.md), accepted. The new `core` methods are domain-agnostic user and session storage, as the seam asks.
- [ADR-003: Asset Identity and Content Revisions](../../../docs/decisions/ADR-003-asset-identity-and-content-revisions.md), accepted
- [ADR-004: Database Engine and Schema Strategy](../../../docs/decisions/ADR-004-database-engine-and-schema.md), accepted. No schema change.
- [ADR-005: v2 Visual Design System](../../../docs/decisions/ADR-005-visual-design-system.md), accepted. No UI change.
- [ADR-006: Declarative Processing, Selectors, and Policy Deferral](../../../docs/decisions/ADR-006-rule-engine-deferral.md), accepted
- [ADR-007: Security and Plugin Trust Model](../../../docs/decisions/ADR-007-security-and-plugin-trust.md), accepted. Its Decision already says the operator can create the administrator from the box, and its first amendment says the operator closes the claim window that way. This change builds that command and amends nothing.
- [ADR-008: Metadata Envelope, Facets, and Write Authority](../../../docs/decisions/ADR-008-facet-schema-and-write-authority.md), accepted
- [ADR-009: Backend Language and Runtime](../../../docs/decisions/ADR-009-backend-language.md), accepted
- [ADR-010: Migration Strategy — Clean Break with a One-Way Importer](../../../docs/decisions/ADR-010-migration-strategy.md), accepted
- [ADR-011: Storage Tree Layout and Cross-Filesystem Moves](../../../docs/decisions/ADR-011-storage-tree-layout.md), accepted
- [ADR-012: Embedded Scripting Engine](../../../docs/decisions/ADR-012-embedded-scripting-engine.md), proposed. Not in force and not relied on.
- [ADR-013: Development Environment](../../../docs/decisions/ADR-013-development-environment.md), accepted. No new crate, so the Nix shell is unchanged.
- [ADR-014: Managed Storage Identity](../../../docs/decisions/ADR-014-managed-storage-identity.md), accepted. The command never pairs or checks storage.
- [ADR-015: Opaque Database-Backed Sessions](../../../docs/decisions/ADR-015-opaque-database-sessions.md), accepted. Its Consequences name administrator recovery as a row delete that `siderealctl`, a separate process, can perform. The reset is that delete.

## New Durable ADRs Created

- None - no major durable architectural decisions were introduced.
