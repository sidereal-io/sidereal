## Why

Rust in this repo is frozen at 1.85.0, 13 releases behind the current stable release, 1.98.1. Nothing updates it, and `AGENTS.md` states "Stable Rust 1.85" as a durable rule. The project wants to stay on current stable Rust, the same way it already takes new Node releases.

An exact pin can't be kept current without manual work. The Nix shell looks Rust up in the `rust-overlay` snapshot locked in `flake.lock`. A Dependabot bump of `rust-toolchain.toml` to a new release would fail its checks until someone merged the weekly `flake.lock` update first and rebased it. That would happen at almost every Rust release, and the maintainer rejected that routine.

## What Changes

- `server/rust-toolchain.toml` names the `stable` channel instead of an exact release. It keeps the `rustfmt` and `clippy` components.
- In the Nix shell and in CI, `flake.lock` decides the exact Rust release. New Rust releases therefore arrive in the weekly Dependabot `flake.lock` pull request. Node already works this way: `.nvmrc` names major version 26, and `flake.lock` picks the exact 26.x.y.
- The repo moves from Rust 1.85.0 to 1.98.1, the release in today's `flake.lock`. On 2026-09-30 the server gates passed on 1.98.1 with no code changes.
- No file names a Rust version any more:
  - `server/Cargo.toml` loses its `rust-version` field;
  - `server/Dockerfile` builds from `rust:1-slim-bookworm` and installs the toolchain that `rust-toolchain.toml` names;
  - `server/README.md` and `AGENTS.md` stop stating 1.85.
- `AGENTS.md` states the new rule: Rust follows the latest stable release, and `flake.lock` decides the exact release in the Nix shell and CI.
- `CONTRIBUTING.md` tells contributors without Nix to run `rustup update` to match CI. It also says what to do when a new Rust release brings clippy lints that fail the weekly `flake.lock` pull request.
- **BREAKING** for contributors without Nix: rustup no longer selects one exact release. Each contributor gets the stable release they last installed, which can differ from CI's release for a few days.
- Unchanged: the Dependabot configuration. It gains no `rust-toolchain` entry, because the `flake.lock` update already carries Rust.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `dev-environment`: Rust stops being the one tool with an exact pin file.
  - "The shell provides the pinned toolchain" takes Rust's exact release from `flake.lock` instead of from `rust-toolchain.toml`.
  - "The Rust version has one source of truth" becomes a rule that Rust follows the latest stable release and that no file names a Rust version.
  - "The shell turns on when you enter the repo" keeps its reload scenario, but no longer assumes `channel` holds an exact release.
  - "The flake check reports the shell's tool versions" adds `rustc --version`, because a `flake.lock` update can now change the Rust release.

## Impact

- **Code and config:** `server/rust-toolchain.toml`, `server/Cargo.toml`, `server/Dockerfile`, and the comment in `nix/toolchains.nix`.
- **Docs:** `AGENTS.md` (which `CLAUDE.md` links to), `CONTRIBUTING.md`, and `server/README.md`.
- **Decision record:** ADR-013 says that for Rust, the Nix shell and rustup "read the same exact-patch pin file, so they can't drift". This change reverses that statement, so it needs a decision record.
- **CI:** the flake check in `nix.yml` also prints `rustc --version`. The triggers don't change: `nix.yml` and `v2.yml` already run when `server/rust-toolchain.toml` or `flake.lock` changes.
- **Dependabot:** each weekly `flake.lock` pull request can now change the Rust release. Weekly Cargo updates can no longer fail because a crate needs a newer compiler than 1.85.
- **Contributors:** the first shell load after this change downloads Rust 1.98.1. Cargo then rebuilds `server/target` once.
