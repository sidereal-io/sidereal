# ADR Review Manifest

- Status: completed
- Review date: 2026-10-09

## Review Summary

ADR review completed for this change. The design changes scanner configuration and baseline migration, without changing application architecture or choosing a new provider.

The highest ADR sequence is 013. No ADR declares a Supersedes field, so the supersession graph has no edges.

ADR-012 remains Proposed. This change does not implement its scripting decision.

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
- ADR-010: Migration Strategy — Clean Break with a One-Way Importer.
- ADR-011: Storage Tree Layout and Cross-Filesystem Moves.
- ADR-013: Development Environment.

The review also read the ADR template and Proposed ADR-012 for context.

## New Durable ADRs Created

None. No major durable architectural decisions were introduced, and no new repository-level ADR files were created.

Fixed categories and SARIF upload ownership remain visible in workflow configuration. Reversing them requires another baseline migration, not an application or infrastructure redesign.
