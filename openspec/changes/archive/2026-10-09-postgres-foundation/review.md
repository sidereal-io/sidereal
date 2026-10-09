## Review Metadata

- Review: bounded timing-metadata exception, 2026-10-09
- Reviewer: Codex, in-session; weaker than the independent review
- Prior independent review: [round two](review-v2.md), APPROVE
- Scope: the owner's accepted timing exception in design.md and specs/postgres-foundation/spec.md; other planning behavior is unchanged
- Owner decision: https://github.com/sidereal-io/sidereal/issues/394#issuecomment-6073900184

## Decision reviewed

Keep sqlx's standard migrator. Migration SQL and essential record fields commit atomically; execution_time is best-effort metadata.
This excludes a custom adapter and timing-repair machinery from this story.

## Findings

### Noted: interruption can retain the timing sentinel

A process interrupted after the migration commits can leave execution_time at -1.
The owner accepts this edge case. Compatibility checks ignore timing metadata, and startup preserves existing records rather than repairing them.
This does not indicate a partially applied schema migration.

### Noted: a timing update can fail after a successful migration commit

A connection failure during the timing update can make sqlx return an error even though the schema and essential record committed.
Startup still fails safely with HTTP unbound. A later startup validates the committed migration rather than rerunning it.
The owner defers extra recovery behavior until an observed operational problem warrants it.

## Coherence check

Bootstrap still creates the identity, migration table, and initial record in one transaction.
Later migrations use standard sqlx behavior with the documented timing exception.
Repeated-startup and retained-state comparisons still cover every field: startup does not modify the accepted sentinel.
The exception does not relax checksum, version-prefix, success, schema, ownership, or TLS checks.
No new migration or public API contract is added by this wording clarification.

## Review limitation

Automatic approval review rejected transferring local planning artifacts to the external Claude service without explicit payload-and-destination authorization.
This bounded clarification therefore received an in-session check. The prior independent review is retained as historical evidence and is not claimed to cover the revised wording.

## Verdict

VERDICT: APPROVE

No blocking finding remains for the accepted exception. All prior requirements remain in force apart from the explicit best-effort timing exception.

CHANGES_APPLIED: n/a
