<!-- Plain language: Make each record link identify the decision so a reader can find its rationale. Keep this manifest separate from the records themselves. -->

# ADR Review Manifest

- Status: completed
- Review date: 2026-11-24

## Review Summary

Reviewed every record in `docs/decisions/`, including the template, the
Proposed scripting record (ADR-012) and the superseded environment record
(ADR-013). The decisions in design.md change only the library screen. None
is a new fork that is costly to reverse:

- Sending one request per file keeps `POST /api/assets` as it is. The route
  keeps the session guard and origin check of
  [ADR-007](../../../docs/decisions/ADR-007-security-and-plugin-trust.md) and
  [ADR-015](../../../docs/decisions/ADR-015-opaque-database-sessions.md), and
  no server code changes, so the dependency rules of
  [ADR-001](../../../docs/decisions/ADR-001-plugin-boundary.md) and
  [ADR-002](../../../docs/decisions/ADR-002-core-domain-pack-split.md) are
  untouched. A multi-file endpoint can still come later without undoing
  anything here.
- The list of lines, the loop that sends files in turn, and the disabled
  Upload button are screen code. Changing them later touches one component
  and its tests.
- The lines use existing tokens and the existing status style, and the
  DESIGN.md edit is wording only, as
  [ADR-005](../../../docs/decisions/ADR-005-visual-design-system.md)
  requires.

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
