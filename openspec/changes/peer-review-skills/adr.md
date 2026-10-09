# ADR Review Manifest

- Status: completed
- Review date: 2026-10-08

## Review Summary

ADR review completed for this change. The design makes five decisions, and none needs an ADR. Each one lives in a skill, the schema, or the OpenSpec config. Each is cheap to reverse, and the file that holds it says what it does.

- **Code review runs after apply, not as an artifact.** Two guidance entries in `openspec/config.yaml` hold it: one for apply, and one for archive. Moving it costs two edits.
- **`peer-plan-review` holds the method, and the schema keeps the OpenSpec checks.** Both are text, and the removed skills can be restored from git.
- **A review runs in-session only when no other family is installed.** It is one paragraph in each review skill.
- **Each review skill holds its own reviewer table.** The tables are text in two skills that agents read. Changing the order or a fallback is a text edit.
- **Review rules live in a root `REVIEW.md`, and `AGENTS.md` points to it.** It is one file and one sentence. Moving the rules elsewhere costs two edits.

## In-Force ADRs Reviewed

- ADR-001 Plugin Contract and Execution Profiles (accepted). This change touches no plugin contract.
- ADR-002 Core / Domain-Pack Seam (accepted). This change touches no Rust crate.
- ADR-003 Asset Identity and Content Revisions (accepted)
- ADR-004 Database Engine and Schema Strategy (accepted)
- ADR-005 v2 Visual Design System (accepted). This change touches no screen or token.
- ADR-006 Declarative Processing, Selectors, and Policy Deferral (accepted)
- ADR-007 Security and Plugin Trust Model (accepted). The reviewers run read-only, and their findings are checked against the code before anyone acts on them.
- ADR-008 Metadata Envelope, Facets, and Write Authority (accepted)
- ADR-009 Backend Language and Runtime (accepted)
- ADR-010 Migration Strategy (accepted). This change touches neither the `v0.x` line nor the importer.
- ADR-011 Storage Tree Layout and Cross-Filesystem Moves (accepted)
- ADR-012 Embedded Scripting Engine (proposed, not in force)
- ADR-013 Development Environment (accepted). The review tools are not added to the Nix shell. The skills check for them at run time and fall back when one is missing.

## New Durable ADRs Created

- None. No major durable architectural decisions were introduced, and no new repository-level ADR files were created.
