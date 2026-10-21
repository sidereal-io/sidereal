# ADR Review Manifest

- Status: completed
- Review date: 2026-10-21

## Review Summary

ADR review completed for this change after reviewing ADR-001 through ADR-014 and the repository template.
No ADR declares a `Supersedes` relationship; the supersession graph has no edges.
The highest sequence number is ADR-014.
ADR-012 remains Proposed and does not govern this deployment change.

The design separates Compose entry files, reuses existing database policy, adds certificate mounts, documents permissions, and isolates deployment verification.
These choices preserve existing architecture and data formats.
The file layout and mount packaging remain inexpensive to revise without migrating user data.

## In-Force ADRs Reviewed

- ADR-001: Plugin Contract and Execution Profiles.
- ADR-002: Core / Domain-Pack Seam.
- ADR-003: Asset Identity and Content Revisions.
- ADR-004: Database Engine and Schema Strategy.
- ADR-005: Visual Design System.
- ADR-006: Declarative Processing, Selectors, and Policy Deferral.
- ADR-007: Security and Plugin Trust.
- ADR-008: Metadata Envelope, Facets, and Write Authority.
- ADR-009: Backend Language.
- ADR-010: Migration Strategy.
- ADR-011: Storage Tree Layout and Cross-Filesystem Moves.
- ADR-013: Development Environment.
- ADR-014: Managed Storage Identity.

## New Durable ADRs Created

None. No major durable architectural decisions were introduced, and no new repository-level ADR files were created.
The change retains PostgreSQL-only persistence, forward-only migrations, the restricted pre-authentication HTTP surface, and database/storage pairing.
