# ADR Review Manifest

- Status: completed
- Review date: 2026-10-27

## Review Summary

ADR review completed for this change. Design decision 11 amends ADR-007 in place, as the story requires. Design decision 4 and the session table are a real fork between signed and opaque tokens that no record held, so it gets a new record. The other design decisions are recoverable from code or follow an existing record: the `core` and `server` split follows ADR-002, the partial unique index and the argon2 parameters are read from the migration and the code, and the cookie parser, error enum, Vite proxy and web state machine are local choices.

## In-Force ADRs Reviewed

- [ADR-001: Plugin contract and execution profiles](../../../docs/decisions/ADR-001-plugin-boundary.md)
- [ADR-002: Core, domain pack split](../../../docs/decisions/ADR-002-core-domain-pack-split.md)
- [ADR-003: Asset identity and content revisions](../../../docs/decisions/ADR-003-asset-identity-and-content-revisions.md)
- [ADR-004: Database engine and schema](../../../docs/decisions/ADR-004-database-engine-and-schema.md)
- [ADR-005: Visual design system](../../../docs/decisions/ADR-005-visual-design-system.md)
- [ADR-006: Rule engine deferral](../../../docs/decisions/ADR-006-rule-engine-deferral.md)
- [ADR-007: Security and plugin trust model](../../../docs/decisions/ADR-007-security-and-plugin-trust.md), amended by this change: the claim is the one unauthenticated mutation and is permitted only while no administrator exists; each prerequisite gates the surface it protects; at-rest key custody is decided by the first feature that stores a provider secret.
- [ADR-008: Facet schema and write authority](../../../docs/decisions/ADR-008-facet-schema-and-write-authority.md)
- [ADR-009: Backend language](../../../docs/decisions/ADR-009-backend-language.md)
- [ADR-010: Migration strategy](../../../docs/decisions/ADR-010-migration-strategy.md)
- [ADR-011: Storage tree layout](../../../docs/decisions/ADR-011-storage-tree-layout.md)
- [ADR-012: Embedded scripting engine](../../../docs/decisions/ADR-012-embedded-scripting-engine.md), proposed
- [ADR-013: Development environment](../../../docs/decisions/ADR-013-development-environment.md)
- [ADR-014: Managed storage identity](../../../docs/decisions/ADR-014-managed-storage-identity.md)

## New Durable ADRs Created

- [ADR-015: Opaque database-backed sessions](../../../docs/decisions/ADR-015-opaque-database-sessions.md): a session is an opaque random token whose SHA-256 hash lives in a PostgreSQL row, not a signed token.
