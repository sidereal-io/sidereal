# ADR Review Manifest

- Status: completed
- Review date: 2026-11-14

## Review Summary

ADR review completed for this change. Brainstorm decisions D1 and D4, carried into the `security-events` spec and design decisions D-4 and D-6, fix how ADR-007's "trust-relevant events are audited" is met: every security event writes a log line, only a claim, an access recovery and a failed sign-in keep a row, and a claim or a recovery fails without its row. This is a privacy posture. An engineer reading only the code could add rows for successful sign-ins, or make the claim's row best effort, without seeing why not. So ADR-007 is amended in place, dated 2026-11-14, with the comparison with Home Assistant, Frigate and Immich, as the user decided. No new record is needed.

The other decisions are recoverable from code and cheap to change:

- One `SecurityEvent` value for the row and the line (D-1), the escaping (D-2), the user agent helper (D-3), the log sink in `Tuning` (D-5), and where the line is printed (D-7, D-8) each live in one place.
- The `security_events` table (D-9) ships in a forward-only migration, but nothing reads it yet, so a later migration can add or change a column at little cost. The spec owns its fields.

## In-Force ADRs Reviewed

- [ADR-001](../../../docs/decisions/ADR-001-plugin-boundary.md): Plugin Contract and Execution Profiles
- [ADR-002](../../../docs/decisions/ADR-002-core-domain-pack-split.md): Core / Domain-Pack Seam
- [ADR-003](../../../docs/decisions/ADR-003-asset-identity-and-content-revisions.md): Asset Identity and Content Revisions
- [ADR-004](../../../docs/decisions/ADR-004-database-engine-and-schema.md): Database Engine and Schema Strategy
- [ADR-005](../../../docs/decisions/ADR-005-visual-design-system.md): v2 Visual Design System
- [ADR-006](../../../docs/decisions/ADR-006-rule-engine-deferral.md): Declarative Processing, Selectors, and Policy Deferral
- [ADR-007](../../../docs/decisions/ADR-007-security-and-plugin-trust.md): Security and Plugin Trust Model, amended by this change: every security event writes a log line, only a claim, an access recovery and a failed sign-in keep a row, and a claim or a recovery fails without its row.
- [ADR-008](../../../docs/decisions/ADR-008-facet-schema-and-write-authority.md): Metadata Envelope, Facets, and Write Authority
- [ADR-009](../../../docs/decisions/ADR-009-backend-language.md): Backend Language and Runtime
- [ADR-010](../../../docs/decisions/ADR-010-migration-strategy.md): Migration Strategy
- [ADR-011](../../../docs/decisions/ADR-011-storage-tree-layout.md): Storage Tree Layout and Cross-Filesystem Moves
- [ADR-013](../../../docs/decisions/ADR-013-development-environment.md): Development Environment
- [ADR-014](../../../docs/decisions/ADR-014-managed-storage-identity.md): Managed Storage Identity
- [ADR-015](../../../docs/decisions/ADR-015-opaque-database-sessions.md): Opaque Database-Backed Sessions

ADR-012 (Embedded Scripting Engine) is Proposed, not in force, and does not bear on this change.

## New Durable ADRs Created

- None - no major durable architectural decisions were introduced. ADR-007 is amended in place.
