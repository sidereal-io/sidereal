# ADR Review Manifest

- Status: completed
- Review date: 2026-09-29

## Review Summary

ADR review completed for this change.

This change makes no decision that is costly to reverse. Every design decision lives in two small configuration files and one documentation section. Undoing them means deleting two Dependabot entries and one workflow file. So no new ADR is needed.

The change fits ADR-013:

- ADR-013 says tools change only when someone updates the lock file in a pull request. Dependabot now opens those pull requests, and a person still merges them.
- ADR-013 says the project must not depend on a hosted service to build. Dependabot proposes updates but plays no part in building. A contributor can still build and update the pins by hand.
- ADR-013 names `nix flake check` as the place for environment checks. The new CI workflow runs it.

## In-Force ADRs Reviewed

- ADR-001: Plugin boundary. Not affected.
- ADR-002: Core and domain pack split. Not affected.
- ADR-003: Asset identity and content revisions. Not affected.
- ADR-004: Database engine and schema. Not affected.
- ADR-005: Frontend continuity. Not affected.
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
