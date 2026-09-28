# ADR Review Manifest

- Status: completed
- Review date: 2026-09-27

## Review Summary

ADR review completed for this change. No decision in `design.md` met the bar, so this change introduces no major durable architectural decision and creates no new repository-level ADR file.

Why each decision falls short:

- **D1 (a generic `just enter` hook)** applies ADR-013 rather than making a new decision. ADR-013 requires modules that can move into a shared flake without changes. D1 keeps the hook free of Sidereal-specific names for that reason. The hook is a few lines in one module, and reversing it costs little.
- **D2 (a stamp and a stale-only mode)**, **D3 (the `enter` recipe)** and **D5 (the direnv watch list)** are small. Each lives in one file, costs little to change, and shows in the code.
- **D4 (turn off the update check)** sets one environment variable. The variable's name explains it.
- **D6 (accept the race)** records an accepted risk. It chooses no lasting structure.

`design.md` records the reasons and the alternatives for all six decisions.

## In-Force ADRs Reviewed

- ADR-013 Development Environment (accepted) — directly relevant. This change follows it: the flake stays built from portable modules, Nix stays optional, and contributors without Nix opt in through their own direnv file. No conflict.
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
