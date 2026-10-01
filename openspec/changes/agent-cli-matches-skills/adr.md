# ADR Review Manifest

- Status: completed
- Review date: 2026-09-30

## Review Summary

ADR review completed for this change. No decision in `design.md` met the bar, so this change introduces no major durable architectural decision and creates no new repository-level ADR file.

Why each decision falls short:

- **D1 (session-start hooks)** and **D2 (`SessionStart`, not `PreToolUse`)** each add a few lines of agent configuration. Deleting or swapping a hook costs little, and the configuration files show what runs.
- **D3 (a `--check` option)** and **D4 (compare only the CLI version and skill folders)** live in one script. Each is cheap to change and readable in the code.
- **D5 (the fix depends on `nix` on `PATH`)** and **D6 (the report speaks to the agent)** decide what one message says. Changing the message costs little.
- **D7 (the hooks call the script directly)** is visible in each hook command.
- **D8 (the rename)** is a file name.

`design.md` records the reasons and the alternatives for all eight decisions.

## In-Force ADRs Reviewed

- ADR-013 Development Environment (accepted) — directly relevant. This change follows it: Nix stays optional, because the check works and suggests a fix without Nix. No wrapper tool sits between contributors and the flake. The change also serves its aim that an agent gets the right tools without working anything out. No conflict.
- ADR-001 Plugin Contract and Execution Profiles (accepted)
- ADR-002 Core / Domain-Pack Seam (accepted)
- ADR-003 Asset Identity and Content Revisions (accepted)
- ADR-004 Database Engine and Schema Strategy (accepted)
- ADR-005 Frontend Continuity (accepted)
- ADR-006 Declarative Processing, Selectors, and Policy Deferral (accepted)
- ADR-007 Security and Plugin Trust Model (accepted)
- ADR-008 Metadata Envelope, Facets, and Write Authority (accepted)
- ADR-009 Backend Language and Runtime (accepted)
- ADR-010 Migration Strategy — Clean Break with a One-Way Importer (accepted)
- ADR-011 Storage Tree Layout and Cross-Filesystem Moves (accepted)
- ADR-012 Embedded Scripting Engine (proposed; not in force, reviewed for context)

ADR-001 to ADR-012 cover the product's architecture and do not touch developer tooling. No ADR has a Supersedes entry, so the supersession graph is empty. The highest number in use is 013.

## New Durable ADRs Created

- None - no major durable architectural decisions were introduced.
