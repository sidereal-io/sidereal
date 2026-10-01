# ADR Review Manifest

- Status: completed
- Review date: 2026-09-30

## Review Summary

ADR review completed for this change. The design makes six decisions (D1–D6), and none needs a new ADR.

- **D1, the stable channel, is a toolchain setting under existing ADRs.** ADR-009 already chooses Rust for the backend. ADR-013 already makes the Nix flake's lock file the pin for every development tool in the Nix shell and CI. It also keeps plain pin files, such as `rust-toolchain.toml` and `.nvmrc`, for contributors without Nix. Which Rust release those files and the lock pick is a setting inside these decisions, not a new fork. The maintainer made this call after the author first drafted a separate ADR.
- **D1 makes one ADR-013 statement false.** ADR-013's Consequences said that for Rust, the Nix shell and rustup "read the same exact-patch pin file, so they can't drift". The change rewrites that consequence in place, so it now covers Rust and Node together. ADR-013 stays accepted, and its decision is unchanged.
- **D2 to D5 are cheap to reverse.** D2 removes one manifest field. D3 changes how one unpublished Dockerfile installs Rust. D4 adds one command to the flake check's log. D5 rewrites docs to match D1.
- **D6 is this step.**

## In-Force ADRs Reviewed

- ADR-001 Plugin contract and execution profiles (accepted). It rules out a Rust dylib plugin ABI, so nothing in the project needs a fixed compiler release.
- ADR-002 Core / domain-pack seam (accepted)
- ADR-003 Asset identity and content revisions (accepted)
- ADR-004 Database engine and schema strategy (accepted)
- ADR-005 Frontend continuity (accepted)
- ADR-006 Declarative processing, selectors, and policy deferral (accepted)
- ADR-007 Security and plugin trust model (accepted)
- ADR-008 Metadata envelope, facets, and write authority (accepted)
- ADR-009 Backend language and runtime (accepted). It chooses Rust, and names no compiler release.
- ADR-010 Migration strategy (accepted)
- ADR-011 Storage tree layout and cross-filesystem moves (accepted)
- ADR-012 Embedded scripting engine (proposed, not in force)
- ADR-013 Development environment (accepted). This change corrects one consequence in place.

No ADR supersedes another. The highest number in use is 013.

## New Durable ADRs Created

- None - no major durable architectural decisions were introduced.
