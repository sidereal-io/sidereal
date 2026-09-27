# ADR Review Manifest

- Status: completed
- Review date: 2026-09-27

## Review Summary

ADR review completed for this change. No decision in `design.md` met the bar, so this change introduces no major durable architectural decision and creates no new repository-level ADR file.

Why each decision falls short:

- **D1 (redirect `XDG_CONFIG_HOME` for one command)** is a stopgap. The recipe's own comment names when to remove it: once OpenSpec reads project-scoped settings.
- **D2 (track the settings file)**, **D3 (delete generated files first)**, **D6 (put the steps in a script)** and **D7 (stay inside the repo)** are small. Each lives in one file or a few lines of shell, costs little to change, and shows in the code.
- **D4 (generate for `agents` only)** depends on the current CLI rendering identical skills for `agents` and `claude`. It is a fact about one tool version, and the drift check in a later story can show when it stops holding.
- **D5 (let Nix own the CLI version)** applies an existing decision rather than making a new one. ADR-013 makes the Nix lock the source of tool versions and keeps Nix optional. It already accepts that the two routes can land on different versions, as they do for Node's patch version. D5 accepts the same drift for the OpenSpec CLI.

`design.md` records the reasons and the alternatives for all seven decisions.

## In-Force ADRs Reviewed

- ADR-013 Development Environment (accepted) — directly relevant. This change follows it: the Nix lock pins the OpenSpec CLI, and contributors without Nix install it through npm. No conflict.
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
