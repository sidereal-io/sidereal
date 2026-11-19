## Why

Contributors and agents get Sidereal's pinned tools only through the Nix flake, which most maintainers cannot change and whose Dependabot updates fail `lets sync --check` until someone fixes them by hand. This change is the first story of #496: it proves a mise route on a contributor's machine and in CI, beside the Nix route, before anything moves onto it.

## What Changes

- **New decision record**, Accepted, that supersedes ADR-013 with the four reasons in #496's Scope. It also says that rustup's newest stable release decides Rust on the mise route (D3).
- **New root `mise.toml`** naming Rust (the `stable` channel, with `rustfmt` and `clippy`), Node (read from `.nvmrc`), pnpm, `just`, `gh`, the `openspec` CLI and Let's Spec. It sets `LETS_SPEC_FLOW` and `OPENSPEC_NO_UPDATE_CHECK=1` (D5), and has an `enter` hook that runs `lets sync`.
- **New committed `mise.lock`** pinning each tool's exact release, and Let's Spec at 0.8.0 (D1). It covers every platform `mise lock` writes by default (D6). Rust is recorded as `stable`, not as a release (D3, D8).
- **`mise.local.toml` is ignored by git**, so a contributor can keep their own mise settings.
- **New `mise` workflow.** On pull requests that change its trigger files (D4), it installs from the lock, prints each tool's version, and fails when `mise.toml` and `server/rust-toolchain.toml` name different Rust channels (D8).
- **The Nix route moves to Let's Spec v0.8.0.** The flake input pins the `v0.8.0` tag until #502 (D2). The same commit carries `flake.lock` and what `lets sync` wrote (D1).
- **`CONTRIBUTING.md`** gains a short "Set up with mise" section, and its "Where versions are set" table names `mise.lock` beside `flake.lock` for the tools both pin (D7).
- Out of scope: moving CI jobs or `justfile` recipes onto mise (#498, #499, #500), removing the Nix route (#502), and a check that fails when the lock drops a platform (#503).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `dev-environment`: gains the mise route — the tools `mise.toml` names, the lock that pins them, the `enter` hook, the offline OpenSpec setting, the ignored personal file, and the `mise` workflow. Its Rust rule changes from "the only Rust version setting is the channel in `server/rust-toolchain.toml`" to "`mise.toml` and `server/rust-toolchain.toml` both name the `stable` channel, and no file names a release" (D8). Both routes use one Let's Spec release (D1).

## Impact

- **New files:** `mise.toml`, `mise.lock`, `.github/workflows/mise.yml`, and a decision record in `docs/decisions/`.
- **Changed files:** `.gitignore`, `flake.nix`, `flake.lock`, `CONTRIBUTING.md`, ADR-013 (status becomes superseded), and the files `lets sync` v0.8.0 rewrites (the `lets-*` skills, the `lets-*` schemas, and the rules block in `AGENTS.md`).
- **Unchanged:** the `ci` workflow, the `justfile`, `.envrc`, `nix/`, `server/rust-toolchain.toml` and `.nvmrc`. Contributors on the Nix route keep working as before, at the newer Let's Spec release.
- **Dependencies:** mise, and the `jdx/mise-action` GitHub Action in the new workflow.
