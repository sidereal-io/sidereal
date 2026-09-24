# ADR Review Manifest

- Status: completed
- Review date: 2026-09-24

## Review Summary

ADR review completed for this change. One decision in `design.md` met the bar: D1, which chooses a native Nix flake over devenv and keeps Nix optional. Reversing it later would change every contributor's setup, CI, and any module shared with other repositories.

The other decisions (D2–D9) did not meet the bar. Each one lives in a few lines of `flake.nix`, `nix/` or `.envrc`, costs little to change, and the code itself shows it. `design.md` records their reasons.

**ADR-013 is Accepted.** The maintainer accepted it on 2026-09-24, after two review rounds.

## In-Force ADRs Reviewed

None of these conflicts with this change. They cover the product's architecture, and this change covers only developer tooling.

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

No ADR has a Supersedes entry, so the supersession graph is empty. The highest number in use was 012.

## New Durable ADRs Created

- [`docs/decisions/ADR-013-development-environment.md`](../../../docs/decisions/ADR-013-development-environment.md) — pin development tools with a native Nix flake built from flake-parts modules, turned on by direnv. Nix is optional for contributors. Status: accepted.
