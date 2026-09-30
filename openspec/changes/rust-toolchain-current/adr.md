# ADR Review Manifest

- Status: completed
- Review date: 2026-09-30

## Review Summary

ADR review completed for this change. The design makes six decisions (D1–D6). One of them, D1, meets the bar for an ADR:

- **D1 is a real fork.** It chooses the stable channel over an exact pin, and over two other ways to keep an exact pin current. A later engineer could plausibly restore an exact pin to gain reproducibility, without knowing that it brings back a manual merge step at almost every Rust release.
- **D1 also contradicts an accepted ADR.** ADR-013's Consequences say that for Rust, the Nix shell and rustup "read the same exact-patch pin file, so they can't drift". That statement stops being true.

The other decisions do not meet the bar:

- D2 removes one manifest field. It is cheap to reverse, and follows from D1.
- D3 changes how one Dockerfile installs Rust. The server image is not published yet.
- D4 adds one command to the flake check's log.
- D5 rewrites docs to match D1.
- D6 is this step.

ADR-013 stays accepted. Its Rust sentence gains a short note that ADR-014 changed it. That is ADR-013's only link to another ADR.

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
- ADR-013 Development environment (accepted). ADR-014 changes its Rust consequence.

No ADR supersedes another. The highest number in use was 013.

## New Durable ADRs Created

- [ADR-014: Rust Follows the Latest Stable Release](../../../docs/decisions/ADR-014-rust-follows-stable.md) — status **proposed**. The maintainer must accept it before implementation starts.
