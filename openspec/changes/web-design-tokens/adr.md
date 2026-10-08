# ADR Review Manifest

- Status: completed
- Review date: 2026-10-07

## Review Summary

ADR review completed for this change. The design makes nine decisions (D1–D9). One of them changes an accepted ADR, and none needs a new ADR file.

- **D9 amends ADR-005 in place, as the maintainer chose.** The amended point: an agent copies tokens into Penpot over the Penpot MCP server, instead of a person importing a generated JSON file. No code ever produced or read that file. `DESIGN.md` stays the source of truth, and Penpot still follows it. One commit makes the edit, with the matching edits to `DESIGN.md`, `AGENTS.md`, and `openspec/config.yaml`.
- **D1 to D3 are a small script and its output.** A contributor can replace the generator without touching any code that reads `tokens.css`. The `--sr-` names come from `DESIGN.md`, not from the script.
- **D4 is a guard.** Its semantic-token check goes away with the first semantic token. Its key and value checks stay. Each states its reason in its error message.
- **D5 follows ADR-013.** ADR-013 pins every tool through a committed file. The linter is pinned by `web/pnpm-lock.yaml`, and the Nix shell gains no tool. Its list of known warnings lives in a script and is cheap to change.
- **D6 is a test.** It encodes the contrast rule that ADR-005 and `DESIGN.md` already state.
- **D7 follows ADR-005's self-hosting rule.** The family names come from `DESIGN.md`. Changing the font source later touches one stylesheet.
- **D8 uses primitives directly.** `DESIGN.md` allows this until a semantic token for the role exists.

## In-Force ADRs Reviewed

- ADR-001 Plugin Contract and Execution Profiles (accepted). This change touches no plugin contract.
- ADR-002 Core / Domain-Pack Seam (accepted). This change touches no Rust crate.
- ADR-003 Asset Identity and Content Revisions (accepted)
- ADR-004 Database Engine and Schema Strategy (accepted)
- ADR-005 v2 Visual Design System (accepted). Amended in place by D9, as described above.
- ADR-006 Declarative Processing, Selectors, and Policy Deferral (accepted)
- ADR-007 Security and Plugin Trust Model (accepted). `DESIGN.md` values flow into CSS. The generator refuses keys and values that could escape a custom property (D4), so the web shell still loads nothing from another origin.
- ADR-008 Metadata Envelope, Facets, and Write Authority (accepted)
- ADR-009 Backend Language and Runtime (accepted)
- ADR-010 Migration Strategy (accepted). The v0.10.x interface is not touched, and none of its values are copied.
- ADR-011 Storage Tree Layout and Cross-Filesystem Moves (accepted)
- ADR-012 Embedded Scripting Engine (proposed, not in force)
- ADR-013 Development Environment (accepted). The new tools are pinned by the `web/` lockfile.

## New Durable ADRs Created

- None. No major durable architectural decisions were introduced. ADR-005 is amended in place, as D9 describes.
