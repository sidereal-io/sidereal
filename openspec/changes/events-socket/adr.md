# ADR Review Manifest

- Status: completed
- Review date: 2026-11-08

## Review Summary

ADR review completed for this change. The change builds on two in-force records: ADR-007 requires WebSocket access that derives from the session and rechecks expiry and revocation, and ADR-015 makes a session a PostgreSQL row looked up by token hash. No design decision in this change is a fork that is costly to reverse.

- Decisions 2 and 3 (the origin check on the upgrade, and the read-only recheck) are behavior. The `instance-access` spec owns them.
- Decisions 1 and 4 to 9 (route placement, the socket loop, injectable timing, the test client, the web shell module, the dev proxy and the guide note) can each be read from the code and changed in one place.
- Detecting revocation by polling rather than by push (D11) can be swapped for `LISTEN/NOTIFY` later without changing the socket's contract.

## In-Force ADRs Reviewed

- [ADR-001](../../../docs/decisions/ADR-001-plugin-boundary.md): Plugin Contract and Execution Profiles
- [ADR-002](../../../docs/decisions/ADR-002-core-domain-pack-split.md): Core / Domain-Pack Seam
- [ADR-003](../../../docs/decisions/ADR-003-asset-identity-and-content-revisions.md): Asset Identity and Content Revisions
- [ADR-004](../../../docs/decisions/ADR-004-database-engine-and-schema.md): Database Engine and Schema Strategy
- [ADR-005](../../../docs/decisions/ADR-005-visual-design-system.md): v2 Visual Design System
- [ADR-006](../../../docs/decisions/ADR-006-rule-engine-deferral.md): Declarative Processing, Selectors, and Policy Deferral
- [ADR-007](../../../docs/decisions/ADR-007-security-and-plugin-trust.md): Security and Plugin Trust Model
- [ADR-008](../../../docs/decisions/ADR-008-facet-schema-and-write-authority.md): Metadata Envelope, Facets, and Write Authority
- [ADR-009](../../../docs/decisions/ADR-009-backend-language.md): Backend Language and Runtime
- [ADR-010](../../../docs/decisions/ADR-010-migration-strategy.md): Migration Strategy
- [ADR-011](../../../docs/decisions/ADR-011-storage-tree-layout.md): Storage Tree Layout and Cross-Filesystem Moves
- [ADR-013](../../../docs/decisions/ADR-013-development-environment.md): Development Environment
- [ADR-014](../../../docs/decisions/ADR-014-managed-storage-identity.md): Managed Storage Identity
- [ADR-015](../../../docs/decisions/ADR-015-opaque-database-sessions.md): Opaque Database-Backed Sessions

ADR-012 (Embedded Scripting Engine) is Proposed, not in force, and does not bear on this change.

## New Durable ADRs Created

- None - no major durable architectural decisions were introduced.
