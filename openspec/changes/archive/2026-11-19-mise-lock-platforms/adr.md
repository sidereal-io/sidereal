<!-- Plain language: Make each record link identify the decision so a reader can find its rationale. Keep this manifest separate from the records themselves. -->

# ADR Review Manifest

- Status: completed
- Review date: 2026-11-19

## Review Summary

ADR review completed for this change. No decision in `design.md` meets the bar for a record. Each of its six decisions lives in one script or one workflow step, costs a single small pull request to reverse, and is explained by the code and `design.md`.

This change applies [ADR-016](../../../docs/decisions/ADR-016-development-environment-with-mise.md), which says one lock file pins every tool except Rust for Linux and macOS alike. The check makes CI enforce that consequence. It does not change the decision.

## In-Force ADRs Reviewed

Only ADR-016 bears on this change. The others cover the product's architecture, and this change covers only developer tooling. None conflicts with it.

- [ADR-001](../../../docs/decisions/ADR-001-plugin-boundary.md) Plugin Contract and Execution Profiles (accepted)
- [ADR-002](../../../docs/decisions/ADR-002-core-domain-pack-split.md) Core / Domain-Pack Seam (accepted)
- [ADR-003](../../../docs/decisions/ADR-003-asset-identity-and-content-revisions.md) Asset Identity and Content Revisions (accepted)
- [ADR-004](../../../docs/decisions/ADR-004-database-engine-and-schema.md) Database Engine and Schema Strategy (accepted)
- [ADR-005](../../../docs/decisions/ADR-005-visual-design-system.md) v2 Visual Design System (accepted)
- [ADR-006](../../../docs/decisions/ADR-006-rule-engine-deferral.md) Declarative Processing, Selectors, and Policy Deferral (accepted)
- [ADR-007](../../../docs/decisions/ADR-007-security-and-plugin-trust.md) Security and Plugin Trust Model (accepted)
- [ADR-008](../../../docs/decisions/ADR-008-facet-schema-and-write-authority.md) Metadata Envelope, Facets, and Write Authority (accepted)
- [ADR-009](../../../docs/decisions/ADR-009-backend-language.md) Backend Language and Runtime (accepted)
- [ADR-010](../../../docs/decisions/ADR-010-migration-strategy.md) Migration Strategy — Clean Break with a One-Way Importer (accepted)
- [ADR-011](../../../docs/decisions/ADR-011-storage-tree-layout.md) Storage Tree Layout and Cross-Filesystem Moves (accepted)
- [ADR-012](../../../docs/decisions/ADR-012-embedded-scripting-engine.md) Embedded Scripting Engine (proposed; not in force, reviewed for context)
- [ADR-013](../../../docs/decisions/ADR-013-development-environment.md) Development Environment (superseded by ADR-016; not in force, reviewed for context)
- [ADR-014](../../../docs/decisions/ADR-014-managed-storage-identity.md) Managed Storage Identity (accepted)
- [ADR-015](../../../docs/decisions/ADR-015-opaque-database-sessions.md) Opaque Database-Backed Sessions (accepted)
- [ADR-016](../../../docs/decisions/ADR-016-development-environment-with-mise.md) Development Environment with mise (accepted)

## New Durable ADRs Created

- None - no major durable architectural decisions were introduced.
