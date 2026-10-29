# ADR Review Manifest

- Status: completed
- Review date: 2026-10-28

## Review Summary

ADR review completed for this change. Design decisions 4 and 7 change the CSRF defence that ADR-007 names, from a token to an origin check with a same-origin default. That is an authentication strategy an engineer could reverse by reading the old ADR text, so ADR-007 is amended in place, as the brainstorm decided (D1). No new record is needed: the trust model and the session shape stand, in ADR-007 and ADR-015. The other decisions are recoverable from code and cheap to change. They are the `/api` router with `route_layer`, `from_fn_with_state`, the `Extension<User>` hand-off, the `AllowedOrigins` config type and the port normalization.

## In-Force ADRs Reviewed

- [ADR-001: Plugin contract and execution profiles](../../../docs/decisions/ADR-001-plugin-boundary.md)
- [ADR-002: Core, domain pack split](../../../docs/decisions/ADR-002-core-domain-pack-split.md)
- [ADR-003: Asset identity and content revisions](../../../docs/decisions/ADR-003-asset-identity-and-content-revisions.md)
- [ADR-004: Database engine and schema](../../../docs/decisions/ADR-004-database-engine-and-schema.md)
- [ADR-005: Visual design system](../../../docs/decisions/ADR-005-visual-design-system.md)
- [ADR-006: Rule engine deferral](../../../docs/decisions/ADR-006-rule-engine-deferral.md)
- [ADR-007: Security and plugin trust model](../../../docs/decisions/ADR-007-security-and-plugin-trust.md), amended by this change: the CSRF defence is an origin check against the request's own host, with an optional exact-match list, instead of a CSRF token; the server sends no cross-origin header.
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
