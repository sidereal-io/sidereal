<!-- Plain language: Make each record link identify the decision so a reader can find its rationale. Keep this manifest separate from the records themselves. -->

# ADR Review Manifest

- Status: completed
- Review date: 2026-11-23

## Review Summary

Reviewed every record in `docs/decisions/`, including the template, the
Proposed scripting record (ADR-012) and the superseded environment record
(ADR-013). The six decisions in design.md apply accepted records. None is a
new fork that is costly to reverse:

- Finding an Asset by content hash uses the hash index that
  [ADR-003](../../../docs/decisions/ADR-003-asset-identity-and-content-revisions.md)
  keeps "for deduplication and verification". It keeps the hash non-unique,
  as ADR-003 and
  [ADR-011](../../../docs/decisions/ADR-011-storage-tree-layout.md) require.
  No schema changes, so reversing it means changing one transaction.
- The per-hash advisory lock is an implementation detail of that
  transaction. It uses PostgreSQL only, as
  [ADR-004](../../../docs/decisions/ADR-004-database-engine-and-schema.md)
  requires.
- Running the Source and publication for every upload keeps ADR-011's
  copy-then-commit-last order and its re-check before reuse.
- The lookup and the lock sit in `core` and know nothing about images, so the
  split in [ADR-002](../../../docs/decisions/ADR-002-core-domain-pack-split.md)
  holds. `plugin-abi` does not change
  ([ADR-001](../../../docs/decisions/ADR-001-plugin-boundary.md)).
- The 409 answer and the library screen's notice are requirements in the
  specs, not architecture. The notice uses existing styles under
  [ADR-005](../../../docs/decisions/ADR-005-visual-design-system.md).
- The upload route keeps the session guard and origin check of
  [ADR-007](../../../docs/decisions/ADR-007-security-and-plugin-trust.md) and
  [ADR-015](../../../docs/decisions/ADR-015-opaque-database-sessions.md).

## In-Force ADRs Reviewed

- [ADR-001 Plugin boundary](../../../docs/decisions/ADR-001-plugin-boundary.md)
- [ADR-002 Core and domain pack split](../../../docs/decisions/ADR-002-core-domain-pack-split.md)
- [ADR-003 Asset identity and content revisions](../../../docs/decisions/ADR-003-asset-identity-and-content-revisions.md)
- [ADR-004 Database engine and schema](../../../docs/decisions/ADR-004-database-engine-and-schema.md)
- [ADR-005 Visual design system](../../../docs/decisions/ADR-005-visual-design-system.md)
- [ADR-006 Rule engine deferral](../../../docs/decisions/ADR-006-rule-engine-deferral.md)
- [ADR-007 Security and plugin trust](../../../docs/decisions/ADR-007-security-and-plugin-trust.md)
- [ADR-008 Facet schema and write authority](../../../docs/decisions/ADR-008-facet-schema-and-write-authority.md)
- [ADR-009 Backend language](../../../docs/decisions/ADR-009-backend-language.md)
- [ADR-010 Migration strategy](../../../docs/decisions/ADR-010-migration-strategy.md)
- [ADR-011 Storage tree layout](../../../docs/decisions/ADR-011-storage-tree-layout.md)
- [ADR-014 Managed storage identity](../../../docs/decisions/ADR-014-managed-storage-identity.md)
- [ADR-015 Opaque database sessions](../../../docs/decisions/ADR-015-opaque-database-sessions.md)
- [ADR-016 Development environment with mise](../../../docs/decisions/ADR-016-development-environment-with-mise.md)

## New Durable ADRs Created

- None - no major durable architectural decisions were introduced.
