# ADR Review Manifest

- Status: completed
- Review date: 2026-10-01

## Review Summary

ADR review completed for this change.

This change makes no decision that is costly to reverse. Every design decision lives in one Dependabot entry and one line of `CONTRIBUTING.md`. Undoing the change means deleting both. Holding TypeScript on version 6 is one `ignore` rule, and deleting the rule ends the hold. So no new ADR is needed.

The change fits ADR-013:

- ADR-013 says pnpm's exact release comes from the project's own committed files. This change leaves the `packageManager` field alone, so Dependabot never changes the pnpm release.
- ADR-013 says the project must not depend on a hosted service to build. Dependabot proposes updates but plays no part in building. A contributor can still update `web/` by hand.

## In-Force ADRs Reviewed

- ADR-001: Plugin boundary. Not affected.
- ADR-002: Core and domain pack split. Not affected.
- ADR-003: Asset identity and content revisions. Not affected.
- ADR-004: Database engine and schema. Not affected.
- ADR-005: Frontend continuity. Not affected. It decides what the new UI starts from, not which package versions it uses.
- ADR-006: Rule engine deferral. Not affected.
- ADR-007: Security and plugin trust. Not affected.
- ADR-008: Facet schema and write authority. Not affected.
- ADR-009: Backend language. Not affected.
- ADR-010: Migration strategy. Not affected.
- ADR-011: Storage tree layout. Not affected.
- ADR-013: Development environment. Consistent, as described above.

ADR-012 (embedded scripting engine) is still Proposed, so it is not in force. It is not affected either. No ADR supersedes another.

## New Durable ADRs Created

- None. No major durable architectural decisions were introduced.
