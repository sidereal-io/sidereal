# ADR Review Manifest

- Status: completed
- Review date: 2026-09-30

## Review Summary

ADR review completed for this change. The design makes seven decisions (D1–D7). None is a real fork that is costly to reverse:

- D1 and D2 change what two `just` recipes contain. Each is a few lines of the `justfile`.
- D3, D4, D5, and D6 change the steps, one cache setting, and the triggers of one CI job. Reverting them restores the rustup steps, and touches no other file.
- D7 chooses which spec holds a requirement. That is spec organization, not architecture.

ADR-013 is the one ADR this change leans on. It makes the Nix flake the pin for every development tool, and keeps Nix optional for contributors. This change is consistent with both parts. CI starts using the flake's shell, and a contributor without Nix still runs `just check` with rustup. No ADR needs an update or a supersession.

## In-Force ADRs Reviewed

- ADR-001 Plugin contract and execution profiles (accepted)
- ADR-002 Core / domain-pack seam (accepted)
- ADR-003 Asset identity and content revisions (accepted)
- ADR-004 Database engine and schema strategy (accepted)
- ADR-005 Frontend continuity (accepted)
- ADR-006 Declarative processing, selectors, and policy deferral (accepted)
- ADR-007 Security and plugin trust model (accepted)
- ADR-008 Metadata envelope, facets, and write authority (accepted)
- ADR-009 Backend language and runtime (accepted)
- ADR-010 Migration strategy (accepted)
- ADR-011 Storage tree layout and cross-filesystem moves (accepted)
- ADR-012 Embedded scripting engine (proposed, not in force)
- ADR-013 Development environment (accepted)

No ADR supersedes another. The highest number in use is 013.

## New Durable ADRs Created

- None - no major durable architectural decisions were introduced.
