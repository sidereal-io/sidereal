## 1. Prerequisites

- [ ] 1.1 The maintainer installs Nix with flakes enabled, using the official installer or Determinate's. Verify that `nix --version` and `nix flake --help` both exit with status 0.

## 2. Flake skeleton

- [ ] 2.1 Create `flake.nix` with three inputs (nixpkgs on `nixos-unstable`, flake-parts and rust-overlay) and three systems (`x86_64-linux`, `aarch64-linux` and `aarch64-darwin`). It imports the three modules under `nix/`. Verify that `nix flake show --json --all-systems` lists `devShells.<system>.default` for all three systems.
- [ ] 2.2 Create `nix/devshell.nix`. It declares the mergeable options `sidereal.shell.packages` and `sidereal.shell.hooks`, builds `devShells.default` from them, and sets `checks.devshell` to that shell (design D4 and D8). Verify that `nix flake check` exits with status 0.
- [ ] 2.3 Generate and commit `flake.lock`. Verify that `nix flake metadata --json` shows a `locked` entry with a `narHash` for every input.

## 3. Tool modules

- [ ] 3.1 Create `nix/toolchains.nix`. It adds Rust from rust-overlay's `fromRustupToolchainFile ./backend/rust-toolchain.toml`, plus `nodejs_24` and `just`. Verify that `nix develop --command rustc --version` reports `1.85.0`, and that `node --version` starts with `v24.`.
- [ ] 3.2 Verify inside the shell that `cargo fmt --version` and `cargo clippy --version` both exit with status 0.
- [ ] 3.3 Create `nix/openspec.nix`. It adds `pkgs.openspec` and contains nothing specific to Sidereal. Verify that `nix develop --command openspec --version` exits with status 0.
- [ ] 3.4 Verify that the Rust version has a single source. Temporarily change `channel` in `backend/rust-toolchain.toml` to another stable version, and confirm that `nix develop --command rustc --version` reports it. Then revert the change.
- [ ] 3.5 Verify that the shell is reproducible. Clone the branch into two different directories, and confirm that `nix eval --raw .#devShells.x86_64-linux.default.drvPath` prints the same path in both.
- [ ] 3.6 Verify that `nix flake check` fails on a broken shell. Temporarily add a package name that doesn't exist to `nix/toolchains.nix`, and confirm that `nix flake check` exits with a non-zero status. Then revert the change.

## 4. direnv integration

- [x] 4.1 Create `.envrc`, following design D7:
  - wrap the Nix steps in a condition on `has nix`;
  - inside it, load nix-direnv 3.2.0 from its release URL with a committed `sha256` hash;
  - inside it, `watch_file` `backend/rust-toolchain.toml` and `nix/*.nix`;
  - inside it, run `use flake`;
  - after the condition, load `.envrc.local` last with `source_env_if_exists`.

  Verify that `direnv allow`, then entering the repo, gives the pinned `rustc --version`, and that leaving the repo restores the previous version. **Verified**: entering gave `rustc 1.85.0`; leaving restored `1.97.1` (the rustup toolchain) and direnv reported "unloading".
- [x] 4.2 Add `.direnv/` and `.envrc.local` to `.gitignore`. Verify that `git status --porcelain` lists neither after `.envrc.local` and `.direnv/` exist. **Verified**: with both present, `git status --porcelain` stayed clean of them.
- [x] 4.3 Verify the hash check. Temporarily change one character of the nix-direnv hash in `.envrc`, then run `direnv allow`. Confirm that direnv stops with an error and doesn't load the script. Then revert the change. **Verified**: direnv reported "error hash mismatch", and `rustc` stayed the non-Nix version.
- [x] 4.4 Verify `.envrc.local`:
  - with `export SIDEREAL_TEST_VAR=1`, the variable equals `1` inside the repo;
  - with `export PATH="$PWD/.local-bin:$PATH"`, the first `PATH` entry is the repo's `.local-bin`;
  - delete `.envrc.local` afterwards.

  **Verified**: both held; `PATH` led with `.local-bin`.
- [x] 4.5 Verify the reload triggers:
  - after `touch backend/rust-toolchain.toml`, `direnv export bash` prints export statements;
  - `direnv status` lists `backend/rust-toolchain.toml` and each file under `nix/` as watched.

  **Verified**: `direnv status` listed `backend/rust-toolchain.toml`, `nix/devshell.nix`, `nix/openspec.nix` and `nix/toolchains.nix` as loaded watches.
- [x] 4.6 Verify that `.env` stays unloaded. Confirm that a variable defined only in `.env` is unset in the shell inside the repo. **Verified**: `IMMICH_URL` (defined only in `.env`) stayed unset.
- [x] 4.7 Verify the route without Nix. In a shell where `nix` isn't on `PATH` (for example a container with direnv but no Nix, or a `PATH` with Nix removed), enter the repo. Confirm that direnv reports no error, and that `env` without its `DIRENV_` lines matches the output from before entering. **Verified**: with `nix` stripped from `PATH`, direnv logged only the load line, and the only environment differences were `DIRENV_*` variables.

## 5. Node version alignment

- [ ] 5.1 Change `.nvmrc` to contain `24` and a trailing newline. Verify with `cat .nvmrc`.
- [ ] 5.2 Replace "Node 20+" and "Node.js 20+" with Node 24 in `AGENTS.md`, `backend/README.md` and `CONTRIBUTING.md`. Verify that `grep -nE 'Node(\.js)? 20' AGENTS.md backend/README.md CONTRIBUTING.md` finds no match.

## 6. Contributor documentation

- [ ] 6.1 Add a "Development environment" section to `CONTRIBUTING.md` covering:
  - the optional Nix route: install Nix with flakes, then run `direnv allow`, or use `nix develop` without direnv;
  - the route without Nix: rustup, a Node version manager that reads `.nvmrc`, and `just`;
  - the one-time `npm rebuild` for anyone who used another Node major version;
  - the one-time full rebuild of `backend/target`;
  - that `.envrc.local` holds personal settings;
  - that flakes see only files tracked by git.

  Verify that a reader can follow each route from the text alone.
- [ ] 6.2 Add one paragraph to `AGENTS.md` and to `backend/README.md` saying that the repo provides an optional Nix shell and that `just check` works in it. Verify that the paragraph links to the `CONTRIBUTING.md` section.

## 7. End-to-end verification

- [ ] 7.1 Run `nix develop --command just check` at the repo root. Verify that it exits with status 0.
- [ ] 7.2 Verify the route without Nix still passes. With rustup and Node 24 from `.nvmrc`, `just check` exits with status 0 outside the Nix shell.
- [ ] 7.3 Run `npm rebuild` inside the shell, then start the v0.10.x server with `npm run dev:server`. Verify that `better-sqlite3` loads without a `NODE_MODULE_VERSION` error.
- [ ] 7.4 Run `openspec validate nix-dev-shell --strict`. Verify that it reports the change as valid.
