# ADR Review Manifest

- Status: completed
- Review date: 2026-10-06

## Review Summary

ADR review completed for this change. No major durable architectural decisions were introduced, and no new repository-level ADR files were created.

The design's decisions are repository maintenance that is cheap to reverse. Reverting the merge restores every removed file, and the archive tag changes nothing when kept or deleted. The one durable policy, that v0 lives only on the `v0.x` branch, carries out ADR-010's decision that the old app goes to maintenance and retires at cutover. It adds no new fork.

## In-Force ADRs Reviewed

No ADR supersedes another. The highest number in use is 013.

- ADR-001 Plugin boundary (accepted). No effect.
- ADR-002 Core, domain pack split (accepted). No effect.
- ADR-003 Asset identity and content revisions (accepted). No effect.
- ADR-004 Database engine and schema (accepted). No effect.
- ADR-005 Visual design system (accepted). No effect. Its references to the v0.10.x interface stay as written.
- ADR-006 Rule engine deferral (accepted). No effect.
- ADR-007 Security and plugin trust (accepted). No effect.
- ADR-008 Facet schema and write authority (accepted). No effect.
- ADR-009 Backend language (accepted). No effect.
- ADR-010 Migration strategy (accepted). This change follows it: the old app stays in maintenance on `v0.x`. The importer it describes must read the released v0.10.x schema, not the unreleased schema this change archives.
- ADR-011 Storage tree layout (accepted). No effect.
- ADR-012 Embedded scripting engine (proposed). Not in force. No effect.
- ADR-013 Development environment (accepted). This change keeps `.nvmrc`, because ADR-013 requires plain pin files for contributors without Nix. Its description of "two stacks" stays as written, because Accepted ADRs record the decision as made.

## New Durable ADRs Created

- None - no major durable architectural decisions were introduced.
