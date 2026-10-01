# ADR Review Manifest

- Status: completed
- Review date: 2026-09-30

## Review Summary

ADR review completed for this change. The design makes nine decisions (D1–D9), and none needs a new ADR.

- **D2, pnpm pinned in `packageManager`, is a toolchain setting under ADR-013.** ADR-013 makes Nix the pinned environment and keeps plain pin files for contributors without Nix. Which package manager a new app uses, and which file pins its exact release, is a setting inside that decision. A new app could switch package managers by regenerating one lockfile.
- **D2 makes one ADR-013 statement too broad.** ADR-013's Consequences said that every tool version comes from one committed lock file. pnpm in `web/` runs the release that `web/package.json` names, and `web/pnpm-lock.yaml` verifies it. The change rewrites that consequence in place to cover tools that switch themselves. ADR-013 stays accepted, and its decision is unchanged.
- **D1 and D5 to D7 follow ADR-005.** ADR-005 calls for a new shell whose first screen does something real. D5 leaves the data-fetching library open for the first data screen, so this change does not decide it early.
- **D6 respects ADR-007.** The same-origin dev proxy needs no CORS or CSRF setting on the server, so it decides none of ADR-007's open rules.
- **D3, D4, D8, and D9 are cheap to reverse.** Each is a few lines in the `justfile`, `nix/toolchains.nix`, or the flake check workflow.

## In-Force ADRs Reviewed

- ADR-001 Plugin boundary (accepted)
- ADR-002 Core / domain pack split (accepted)
- ADR-003 Asset identity and content revisions (accepted)
- ADR-004 Database engine and schema (accepted)
- ADR-005 Frontend continuity (accepted). The web shell is the new shell this ADR calls for.
- ADR-006 Rule engine deferral (accepted)
- ADR-007 Security and plugin trust (accepted). This change adds no route and no cross-origin access to the server.
- ADR-008 Facet schema and write authority (accepted)
- ADR-009 Backend language (accepted)
- ADR-010 Migration strategy (accepted). The v0.10.x stack keeps running, with renamed `just` recipes.
- ADR-011 Storage tree layout (accepted)
- ADR-012 Embedded scripting engine (proposed, not in force)
- ADR-013 Development environment (accepted). This change corrects one consequence in place.

No ADR supersedes another. The highest number in use is 013.

## New Durable ADRs Created

- None - no major durable architectural decisions were introduced.
