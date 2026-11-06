# ADR Review Manifest

- Status: completed
- Review date: 2026-11-02

## Review Summary

ADR review completed for this change. Design decisions 2, 5 and 6 together fix the trust model for reverse proxies: only proxies named in `SIDEREAL_TRUSTED_PROXIES` are believed, unset trusts nobody, and forwarded headers from any other peer are refused rather than ignored. ADR-007 already names reverse proxies and shared networks as standard risks and defines the session cookie's attributes, and an engineer reading only the code could relax this to Immich's private-range default or to silent ignoring without seeing the cost. So ADR-007 is amended in place, as the brainstorm decided (D9, D15). No new record is needed. The other decisions are recoverable from code and cheap to change: `ConnectInfo` wiring, the `layer` placement, the `Forwarded` extension, hand-rolled CIDR matching, the Compose bind address and the pass-through.

## In-Force ADRs Reviewed

- [ADR-001: Plugin contract and execution profiles](../../../docs/decisions/ADR-001-plugin-boundary.md)
- [ADR-002: Core, domain pack split](../../../docs/decisions/ADR-002-core-domain-pack-split.md)
- [ADR-003: Asset identity and content revisions](../../../docs/decisions/ADR-003-asset-identity-and-content-revisions.md)
- [ADR-004: Database engine and schema](../../../docs/decisions/ADR-004-database-engine-and-schema.md)
- [ADR-005: Visual design system](../../../docs/decisions/ADR-005-visual-design-system.md)
- [ADR-006: Rule engine deferral](../../../docs/decisions/ADR-006-rule-engine-deferral.md)
- [ADR-007: Security and plugin trust model](../../../docs/decisions/ADR-007-security-and-plugin-trust.md), amended by this change: forwarded headers are believed only from proxies named in configuration, unset trusts nobody, and forwarded headers from any other peer are refused.
- [ADR-008: Facet schema and write authority](../../../docs/decisions/ADR-008-facet-schema-and-write-authority.md)
- [ADR-009: Backend language](../../../docs/decisions/ADR-009-backend-language.md)
- [ADR-010: Migration strategy](../../../docs/decisions/ADR-010-migration-strategy.md)
- [ADR-011: Storage tree layout](../../../docs/decisions/ADR-011-storage-tree-layout.md)
- [ADR-012: Embedded scripting engine](../../../docs/decisions/ADR-012-embedded-scripting-engine.md), proposed
- [ADR-013: Development environment](../../../docs/decisions/ADR-013-development-environment.md)
- [ADR-014: Managed storage identity](../../../docs/decisions/ADR-014-managed-storage-identity.md)
- [ADR-015: Opaque database-backed sessions](../../../docs/decisions/ADR-015-opaque-database-sessions.md)

## New Durable ADRs Created

- None - no major durable architectural decisions were introduced. ADR-007 is amended in place.
