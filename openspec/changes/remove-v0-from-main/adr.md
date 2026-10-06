# ADR Review Manifest

- Status: completed
- Review date: 2026-10-06

## Review Summary

ADR review completed for this change. It creates no new repository-level ADR.

The design's decisions are repository maintenance that is cheap to reverse. Reverting the merge restores every removed file, and the archive tag changes nothing whether it is kept or deleted.

The rule that v0 fixes land on `v0.x` only is the maintainer's working policy, not an architectural fork. The two lines share no code, so `main` cannot take a v0 fix anyway. A bug in both lines is fixed separately in each. The rule carries out ADR-010's decision that the old app goes to maintenance and retires at cutover.

## In-Force ADRs Reviewed

No ADR supersedes another. The highest number in use is 013. ADR-012 is Proposed, so it is not in force.

Two ADRs shape this change:

- **ADR-010, Migration strategy (accepted).** The old app stays in maintenance on `v0.x` until cutover. The importer it describes reads the latest released v0 schema at cutover, not the unreleased schema this change archives.
- **ADR-013, Development environment (accepted).** It requires the plain pin files that contributors without Nix use, so this change keeps `.nvmrc`. Its wording about "two stacks" stays as written, because Accepted ADRs record the decision as it was made.

ADR-001 to ADR-009 and ADR-011 cover the plugin model, the data model, storage, security, the design system, and the backend language. This change touches none of them. ADR-005 refers to the v0.10.x interface, and that wording stays.

## New Durable ADRs Created

- None - no major durable architectural decisions were introduced.
