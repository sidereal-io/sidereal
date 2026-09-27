# ADR Review Manifest

- Status: completed
- Review date: 2026-09-27

## Review Summary

ADR review completed for this change. No decision in `design.md` met the bar, so this change introduces no major durable architectural decision and creates no new repository-level ADR file.

Why each decision falls short:

- **D1 (redirect `XDG_CONFIG_HOME` for one command)** is a stopgap. The recipe's own comment names when to remove it: once OpenSpec reads project-scoped settings.
- **D2 (write the config at run time)**, **D3 (delete generated files first)** and **D6 (one bash script)** each live in a few lines of the `justfile` or `.gitignore`. They cost little to change, and the code shows them.
- **D4 (generate for `agents` only)** depends on the current CLI rendering identical skills for `agents` and `claude`. It is a fact about one tool version, and the drift check in a later story can show when it stops holding.
- **D5 (pin the CLI version in the `justfile`)** applies an existing decision rather than making a new one. ADR-013 keeps Nix optional and requires both routes, with Nix and without, to keep working through plain pin files. D5 adds the same kind of pin file for the OpenSpec CLI that `backend/rust-toolchain.toml` and `.nvmrc` already give Rust and Node.

`design.md` records the reasons and the alternatives for all six decisions.

## In-Force ADRs Reviewed

- ADR-013 Development Environment (accepted) — directly relevant. This change follows it: the Nix shell stays the installer for contributors with Nix, and a plain pin covers contributors without it. No conflict.
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
