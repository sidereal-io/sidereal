# ADR Review Manifest

- Status: completed
- Review date: 2026-10-16

## Review Summary

ADR review completed for this change.
The existing ADRs contain no `Supersedes` edges; the highest existing sequence was 013.
ADR-012 remains proposed and concerns scripting, which this change does not use.
The storage identity contract is the durable architectural fork.
Compose overrides and probe mechanics remain implementation choices in the design.
The operator accepted the revised startup initialization decision on 2026-10-19.

## In-Force ADRs Reviewed

- ADR-001: Plugin Contract and Execution Profiles.
- ADR-002: Core / Domain-Pack Seam.
- ADR-003: Asset Identity and Content Revisions.
- ADR-004: Database Engine and Schema Strategy.
- ADR-005: v2 Visual Design System.
- ADR-006: Declarative Processing, Selectors, and Policy Deferral.
- ADR-007: Security and Plugin Trust Model.
- ADR-008: Metadata Envelope, Facets, and Write Authority.
- ADR-009: Backend Language.
- ADR-010: Migration Strategy.
- ADR-011: Storage Tree Layout and Cross-Filesystem Moves.
- ADR-013: Development Environment.

## New Durable ADRs Created

- [ADR-014: Managed Storage Identity](../../../docs/decisions/ADR-014-managed-storage-identity.md).
  Status: accepted. The operator approved the revised startup initialization decision on 2026-10-19.
  This ADR supersedes no existing decision.
