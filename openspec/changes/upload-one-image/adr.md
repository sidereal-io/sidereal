# ADR Review Manifest

- Status: completed
- Review date: 2026-11-21

## Review Summary

Reviewed every record in `docs/decisions/`, including the template, the
Proposed scripting record and the superseded environment record. The five
design decisions implement accepted boundaries rather than introduce a new
durable architectural fork:

- Source execution and the image pack apply ADR-001 and ADR-002. The narrow
  execution interface remains pre-freeze and can evolve with the later Operator.
- Asset/version persistence implements ADR-003 with ADR-004 migrations.
- Staging, object sharing and publication implement ADR-011; lazy directory
  creation preserves ADR-014 pairing.
- API composition retains ADR-007 and ADR-015 access checks.
- The library uses ADR-005 without a new UI stack or token system.

The revisions after plan review round 1 (D10–D14) add no fork either. The
web-root overlap refusal and the re-hash before reuse enforce ADR-007's
trust boundary and ADR-011's integrity promise. Semantic tokens follow
ADR-005 and DESIGN.md's own tier rules. The isolated e2e stack is test
tooling within ADR-016's mise tasks.

ADR-012 remains Proposed and unused. ADR-013 is superseded by ADR-016; the
plan follows the latter. No existing record was changed.

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

## New Durable ADRs Created

- None — the durable choices are already made by the records above. The file
  cap and recognition boundary are confirmed requirements. Multipart shapes,
  test seams and component organization are implementation choices whose
  rationale is retained in design.md.
