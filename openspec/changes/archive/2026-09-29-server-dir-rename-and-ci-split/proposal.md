## Why

The v2 Rust workspace lives in `backend/`, but the v2 layout puts it in `server/`, beside a future `web/`. Every M1 story writes into this tree, so renaming it now avoids rebases and stale paths later. CI has a second problem: a pull request that changes only v2 code still runs the whole v0.10.x pipeline and builds a v0.10.x image.

## What Changes

- **BREAKING (contributors only):** the Rust workspace moves from `backend/` to `server/`. The rename changes paths, not behavior. The crate names stay the same, including `sidereal-server`.
- **BREAKING (contributors only):** the `just backend` recipe becomes `just server`.
- Every live reference to the old path changes to `server/`. This covers the justfile, `.envrc`, the Nix toolchain module, `.gitignore`, `.github/dependabot.yml`, the Nix CI trigger, the arch lint and pack comments, the Dockerfile comments, and the contributor docs.
- The v2 workflow `backend-rs.yml` becomes `v2.yml`, with one job named `server`. It runs on changes to `server/**`, the `justfile`, or itself, and keeps today's steps.
- The v0.10.x workflows `ci.yml` and `docker-build-push.yml` skip pull requests that change only `server/**`, `web/**`, `openspec/**`, or `docs/**`.
- The CodeQL job moves out of `ci.yml` into its own `codeql.yml`, with no path filter. CodeQL then keeps scanning every pull request, including v2 pull requests. The v2 web app in `web/` will be TypeScript, which CodeQL analyzes.
- Unchanged: archived OpenSpec changes, and the word "backend" used as a concept in ADRs and architecture docs.

## Capabilities

### New Capabilities

- `ci`: which CI workflows run for which pull requests. It covers the split between v0.10.x and v2 pipelines, and code scanning on every pull request.

### Modified Capabilities

- `dev-environment`: requirements that name `backend/rust-toolchain.toml`, `backend/Cargo.lock`, `backend/README.md`, or the `/backend` Dependabot directory now name the `server/` paths.

## Impact

- **Code and config:** `backend/` moves to `server/`. Also affected: `justfile`, `.envrc`, `nix/toolchains.nix`, `.gitignore`, `.github/dependabot.yml`, and `.github/workflows/` (`backend-rs.yml`, `ci.yml`, `docker-build-push.yml`, `nix.yml`, and the new `codeql.yml`).
- **Docs:** `AGENTS.md`, `CONTRIBUTING.md`, `server/README.md`, and `openspec/discovery.md`.
- **Contributors:** after pulling, each contributor deletes the ignored `backend/target/` that git leaves behind. Cargo rebuilds once in `server/target/`.
- **Open work:** #310 edits the same Dependabot, Dockerfile, and `AGENTS.md` lines. Whichever story lands second updates the other's paths. #289 later moves the `server` job's steps into the Nix shell.
