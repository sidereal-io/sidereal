# ADR Review Manifest

- Status: completed
- Review date: 2026-10-20

## Review Summary

ADR review completed for this change.
The existing ADRs contain no `Supersedes` edges. The highest existing sequence is 014.
ADR-012 remains proposed. It concerns scripting, which this change does not use.

The design makes seven decisions, including how timeouts and TLS errors are classified. None is a costly fork:

- Classifying connection errors is one mapping function. Code review can change it at little cost.
- Readiness change tracking, its two log lines, and the cached answer for an overlapping probe can be read from the code. Adding a logging library later would replace them without touching any data.
- Pointing the image health check at `/readyz` is a one-line change, and rolling it back is the same size.
- Reading certificate files during configuration is a local check with no stored effect.
- The test relay and the verification step are implementation details.

No major durable architectural decisions were introduced, and no new repository-level ADR files were created.

## In-Force ADRs Reviewed

- ADR-001: Plugin Contract and Execution Profiles.
- ADR-002: Core / Domain-Pack Seam. The new readiness types stay domain-agnostic in `core`.
- ADR-003: Asset Identity and Content Revisions.
- ADR-004: Database Engine and Schema Strategy. The change adds no schema and keeps PostgreSQL as the only engine.
- ADR-005: v2 Visual Design System. The change leaves the home screen as it is, so the badge rule is not engaged.
- ADR-006: Declarative Processing, Selectors, and Policy Deferral.
- ADR-007: Security and Plugin Trust Model. Health probes stay unauthenticated, and logs stay redacted.
- ADR-008: Metadata Envelope, Facets, and Write Authority.
- ADR-009: Backend Language and Runtime.
- ADR-010: Migration Strategy — Clean Break with a One-Way Importer.
- ADR-011: Storage Tree Layout and Cross-Filesystem Moves.
- ADR-013: Development Environment.
- ADR-014: Managed Storage Identity. Storage diagnostics stay unchanged.

## New Durable ADRs Created

None.
