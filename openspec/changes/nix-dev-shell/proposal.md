Origin: `openspec/discovery.md`, Track E, story E1 (`nix-dev-shell`). There is no GitHub issue.

## Why

Each contributor and each AI agent builds this repo with whatever tools their machine happens to have. Tool versions drift as a result:

- **Node.** `.nvmrc` says `24.10.0` and CI uses `24.x`. `AGENTS.md`, `backend/README.md` and `CONTRIBUTING.md` say "Node 20+". The maintainer's machine runs Node 26.
- **OpenSpec.** The `openspec` CLI is a global npm install that nothing pins.
- **Rust.** Only Rust is pinned, through `backend/rust-toolchain.toml`.

This change gives the repo one pinned environment that turns on when you enter the directory. Later stories in Track E build on it: generated OpenSpec skills (E2), skills refreshed on entry (E3), and a CI drift check (E4).

## What Changes

- **A Nix flake at the repo root.** It uses flake-parts and provides one development shell with:
  - Rust at the exact version in `backend/rust-toolchain.toml`;
  - Node 24;
  - `just`;
  - the `openspec` CLI.
- **Split into small modules under `nix/`.** The shell setup, the toolchains and `openspec` each get their own module. The `openspec` module stays free of anything specific to Sidereal, so a shared flake can take it over later.
- **A committed `flake.lock`.** It pins every input to an exact revision. Tool versions change only when someone updates the lock in a pull request.
- **An `.envrc` for direnv.** It enters the shell automatically when Nix is installed, and does nothing when Nix is missing. It also loads a contributor's own gitignored `.envrc.local`, if one exists.
- **`nix flake check` builds the shell.** This gives Track E's later CI work somewhere to add checks.
- **Node 24 becomes the documented version everywhere.** `.nvmrc` becomes `24`. `AGENTS.md`, `backend/README.md` and `CONTRIBUTING.md` say Node 24 instead of "Node 20+".
- **`CONTRIBUTING.md` documents Nix as optional.** It describes both routes: with Nix, and with rustup and a Node version manager. It also lists the one-time steps after first entering the shell.

Nothing here is **BREAKING**. Contributors without Nix keep working exactly as today.

## Capabilities

### New Capabilities

- `dev-environment`: the pinned development environment. It covers the tools that contributors and agents get, the environment turning on automatically, and the route for contributors without Nix. Track E's later stories extend this capability.

### Modified Capabilities

None.

## Impact

- **New files:**
  - `flake.nix` and `flake.lock`;
  - `nix/devshell.nix`, `nix/toolchains.nix` and `nix/openspec.nix`;
  - `.envrc`.
- **Changed files:**
  - `.nvmrc`;
  - `.gitignore` (adds `.direnv/` and `.envrc.local`);
  - `AGENTS.md`, `backend/README.md` and `CONTRIBUTING.md`.
- **New inputs:** nixpkgs (`nixos-unstable`, pinned by the lock), flake-parts and rust-overlay. The environment reuses nix-direnv from a pinned URL. Everything comes prebuilt from cache.nixos.org, so no one compiles anything.
- **Contributors who adopt Nix** do two one-time things:
  - run `npm rebuild` if they used another Node major version before, because `better-sqlite3` is a native module built for that version;
  - wait for one full rebuild of `backend/target`.
- **Not affected:**
  - the existing CI workflows;
  - the `justfile` recipes;
  - application code in either stack;
  - OpenSpec skills.
