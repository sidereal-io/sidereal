# ADR Review Manifest

- Status: completed
- Review date: 2026-10-01

## Review Summary

ADR review completed for this change. The design makes seven decisions (D1–D7), and none needs a new ADR. No major durable architectural decisions were introduced, and no new repository-level ADR files were created.

- **D2 to D4, the lint, format, and test tools, are dev dependencies of one app.** A contributor can swap any of them by changing a config file and the lockfile. No runtime code or data depends on them. They are tool settings, not a fork in the architecture.
- **D2's strict rules have one cost under ADR-005.** ADR-005 ports presentational components from the v0.10.x client into the new shell. A ported component must now pass type-aware lint, which v0.10.x never ran. Each port therefore includes lint fixes. That cost is a per-port judgment call, which ADR-005 already expects. It does not change that decision.
- **D2 to D4 follow ADR-013.** ADR-013 pins every tool through a committed file. The new tools are pinned in `web/pnpm-lock.yaml`, and the Nix shell gains no tool.
- **D1, D5, and D6 are cheap to reverse.** Each is a script, a recipe, or a workflow job of a few lines. The shared path filter in D6 follows the epic's choice of one `v2` workflow, which is a CI layout setting, not an ADR.
- **D7 changes no behavior.** It reformats code and fixes lint errors only.

## In-Force ADRs Reviewed

- ADR-001 Plugin boundary (accepted). This change touches no plugin contract.
- ADR-002 Core / domain pack split (accepted). This change touches no Rust crate.
- ADR-003 Asset identity and content revisions (accepted)
- ADR-004 Database engine and schema (accepted)
- ADR-005 Frontend continuity (accepted). Ported components must pass the new lint, as noted above.
- ADR-006 Rule engine deferral (accepted)
- ADR-007 Security and plugin trust (accepted). This change adds no route and no cross-origin access.
- ADR-008 Facet schema and write authority (accepted)
- ADR-009 Backend language (accepted)
- ADR-010 Migration strategy (accepted). The v0.10.x gate stays `npm run check`.
- ADR-011 Storage tree layout (accepted)
- ADR-012 Embedded scripting engine (proposed, not in force)
- ADR-013 Development environment (accepted). The new tools are pinned by the `web/` lockfile.

## New Durable ADRs Created

- None. No major durable architectural decisions were introduced.
