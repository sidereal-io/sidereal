# ADR Review Manifest

- Status: completed
- Review date: 2026-11-25

## Review Summary

ADR review completed for this change. The change adds tests and two requirements for behavior earlier changes already built. None of the five decisions in `design.md` is costly to reverse, so none needs a record.

One point for #526: [ADR-001](../../../docs/decisions/ADR-001-plugin-boundary.md) lets the Asset context generate new assets in time. The requirement "An Operator run makes no Asset" describes the contract as it stands today. #526, which adds derived assets, must change that requirement and its test.

## In-Force ADRs Reviewed

- [ADR-001 — Plugin boundary](../../../docs/decisions/ADR-001-plugin-boundary.md)
- [ADR-002 — Core and domain pack split](../../../docs/decisions/ADR-002-core-domain-pack-split.md)
- [ADR-003 — Asset identity and content revisions](../../../docs/decisions/ADR-003-asset-identity-and-content-revisions.md)
- [ADR-004 — Database engine and schema](../../../docs/decisions/ADR-004-database-engine-and-schema.md)
- [ADR-005 — Visual design system](../../../docs/decisions/ADR-005-visual-design-system.md)
- [ADR-006 — Rule engine deferral](../../../docs/decisions/ADR-006-rule-engine-deferral.md)
- [ADR-007 — Security and plugin trust](../../../docs/decisions/ADR-007-security-and-plugin-trust.md)
- [ADR-008 — Facet schema and write authority](../../../docs/decisions/ADR-008-facet-schema-and-write-authority.md)
- [ADR-009 — Backend language](../../../docs/decisions/ADR-009-backend-language.md)
- [ADR-010 — Migration strategy](../../../docs/decisions/ADR-010-migration-strategy.md)
- [ADR-011 — Storage tree layout](../../../docs/decisions/ADR-011-storage-tree-layout.md)
- [ADR-014 — Managed storage identity](../../../docs/decisions/ADR-014-managed-storage-identity.md)
- [ADR-015 — Opaque database sessions](../../../docs/decisions/ADR-015-opaque-database-sessions.md)
- [ADR-016 — Development environment with mise](../../../docs/decisions/ADR-016-development-environment-with-mise.md)
- [ADR-017 — Facet value rows](../../../docs/decisions/ADR-017-facet-value-rows.md)

## New Durable ADRs Created

- None - no major durable architectural decisions were introduced.
