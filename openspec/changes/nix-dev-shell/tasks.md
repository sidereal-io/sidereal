## 1. Prerequisites

- [x] 1.1 The maintainer installs Nix with flakes enabled, using the official installer or Determinate's. Verify that `nix --version` and `nix flake --help` both exit with status 0. **Verified**: Nix 2.35.2 is installed; both commands exit 0 (with `experimental-features = nix-command flakes` set, since this machine's default `nix.conf` doesn't enable flakes globally — noted below).

## 2. Flake skeleton

- [x] 2.1 Create `flake.nix` with three inputs (nixpkgs on `nixos-unstable`, flake-parts and rust-overlay) and three systems (`x86_64-linux`, `aarch64-linux` and `aarch64-darwin`). It imports the three modules under `nix/`. Verify that `nix flake show --json --all-systems` lists `devShells.<system>.default` for all three systems. **Verified**: listed for all three.
- [x] 2.2 Create `nix/devshell.nix`. It declares the mergeable options `sidereal.shell.packages` and `sidereal.shell.hooks`, builds `devShells.default` from them, and sets `checks.devshell` to that shell (design D4 and D8). Verify that `nix flake check` exits with status 0. **Verified**: exit 0, "all checks passed!".
- [x] 2.3 Generate and commit `flake.lock`. Verify that `nix flake metadata --json` shows a `locked` entry with a `narHash` for every input. **Verified**: `nixpkgs`, `flake-parts`, `flake-parts/nixpkgs-lib`, `rust-overlay` and `rust-overlay/nixpkgs` (follows `nixpkgs`) each have a `locked.narHash`.

## 3. Tool modules

- [x] 3.1 Create `nix/toolchains.nix`. It adds Rust from rust-overlay's `fromRustupToolchainFile ./backend/rust-toolchain.toml`, plus `nodejs_24` and `just`. Verify that `nix develop --command rustc --version` reports `1.85.0`, and that `node --version` starts with `v24.`. **Verified**: `rustc 1.85.0`, `node v24.20.0`.
- [x] 3.2 Verify inside the shell that `cargo fmt --version` and `cargo clippy --version` both exit with status 0. **Verified**: both ran (`rustfmt 1.8.0-stable`, `clippy 0.1.85`).
- [x] 3.3 Create `nix/openspec.nix`. It adds `pkgs.openspec` and contains nothing specific to Sidereal. Verify that `nix develop --command openspec --version` exits with status 0. **Verified**: reported `1.13.1`.
- [x] 3.4 Verify that the Rust version has a single source. Temporarily change `channel` in `backend/rust-toolchain.toml` to another stable version, and confirm that `nix develop --command rustc --version` reports it. Then revert the change. **Verified**: changing the channel to `1.84.1` changed the shell's `rustc` to match; reverting restored `1.85.0`.
- [x] 3.5 Verify that the shell is reproducible. Clone the branch into two different directories, and confirm that `nix eval --raw .#devShells.x86_64-linux.default.drvPath` prints the same path in both. **Verified**: both clones printed the identical `.drv` path.
- [x] 3.6 Verify that `nix flake check` fails on a broken shell. Temporarily add a package name that doesn't exist to `nix/toolchains.nix`, and confirm that `nix flake check` exits with a non-zero status. Then revert the change. **Verified**: exit 1 with the broken package; exit 0 after reverting.

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

- [x] 5.1 Change `.nvmrc` to contain `24` and a trailing newline. Verify with `cat .nvmrc`. **Verified**: contains `24\n`.
- [x] 5.2 Replace "Node 20+" and "Node.js 20+" with Node 24 in `AGENTS.md`, `backend/README.md` and `CONTRIBUTING.md`. Verify that `grep -nE 'Node(\.js)? 20' AGENTS.md backend/README.md CONTRIBUTING.md` finds no match. **Verified**: no match. (The verification regex doesn't span `backend/README.md`'s markdown link syntax around "Node.js"; checked that occurrence by hand too.)

## 6. Contributor documentation

- [x] 6.1 Add a "Development environment" section to `CONTRIBUTING.md` covering:
  - the optional Nix route: install Nix with flakes, then run `direnv allow`, or use `nix develop` without direnv;
  - the route without Nix: rustup, a Node version manager that reads `.nvmrc`, and `just`;
  - the one-time `npm rebuild` for anyone who used another Node major version;
  - the one-time full rebuild of `backend/target`;
  - that `.envrc.local` holds personal settings;
  - that flakes see only files tracked by git.

  Verify that a reader can follow each route from the text alone. **Verified**: added `## 🧰 Development Environment` with numbered "With Nix" and "Without Nix" routes; both steps and both one-time-cost notes read standalone.
- [x] 6.2 Add one paragraph to `AGENTS.md` and to `backend/README.md` saying that the repo provides an optional Nix shell and that `just check` works in it. Verify that the paragraph links to the `CONTRIBUTING.md` section. **Verified**: both paragraphs link to `CONTRIBUTING.md#development-environment`.

## 7. End-to-end verification

- [x] 7.1 Run `nix develop --command just check` at the repo root. Verify that it exits with status 0. **Verified**: exit 0, all cargo checks and the arch lint passed.
- [x] 7.2 Verify the route without Nix still passes. With rustup and Node 24 from `.nvmrc`, `just check` exits with status 0 outside the Nix shell. **Verified**: exit 0. (`just check` runs only the Rust checks; rustup selects 1.85.0 from `backend/rust-toolchain.toml` regardless of the active Node version.)
- [x] 7.3 Run `npm rebuild` inside the shell, then start the v0.10.x server with `npm run dev:server`. Verify that `better-sqlite3` loads without a `NODE_MODULE_VERSION` error. **Verified**: `npm rebuild` succeeded, then `npm run dev:server` logged "Using SQLite database: local.db", ran migrations and started serving — no `NODE_MODULE_VERSION` error.
- [x] 7.4 Run `openspec validate nix-dev-shell --strict`. Verify that it reports the change as valid. **Verified**: "Change 'nix-dev-shell' is valid".

## 8. Node version bump to 26

Groups 1-7 above pinned Node 24. This section moves the pin to 26 (design.md D6,
revised) to match what the maintainer's machine already runs, and closes a gap
`/opsx:verify` found in root `README.md`. A fresh review round must approve this
revision before anyone works these tasks (review.md's prior APPROVE is void once
the plan changed).

- [x] 8.1 Change `nix/toolchains.nix`: `pkgs.nodejs_24` -> `pkgs.nodejs_26`. Verify that `nix develop --command node --version` starts with `v26.`. **Verified**: `v26.10.0`.
- [x] 8.2 Change `.nvmrc` to contain `26` and a trailing newline. Verify with `cat .nvmrc`. **Verified**: `26\n`.
- [x] 8.3 Replace "Node 24" with "Node 26" in `AGENTS.md`, `backend/README.md` and `CONTRIBUTING.md`. **Verified**: all four occurrences updated (including `backend/README.md`'s markdown-link line and CONTRIBUTING.md's two prose mentions).
- [x] 8.4 Fix the stale "Node.js 20+" / "`# v20+`" in the root `README.md` (the `/opsx:verify` finding) to Node 26. **Verified**: both lines now say 26.
- [x] 8.5 Change `.github/workflows/ci.yml`'s `node-version` from `'24.x'` to `'26.x'`. **Verified**: also updated the step's display name ("Setup Node.js 24.x" -> "26.x"), which carried the same stale string and would otherwise have failed task 8.6's grep.
- [x] 8.6 Verify no stale Node 24 or 20 references remain:
  - `grep -rnE 'Node(\.js)?\s?v?(24|20)\b|node:(24|20)|(24|20)\.x|\bv(24|20)\b' AGENTS.md README.md backend/README.md CONTRIBUTING.md .github/workflows/ci.yml` finds no match. The `\bv(24|20)\b` alternative catches a bare version comment like `# v20+`. Without it, task 8.4's fix to that exact `README.md` line would go unverified.
  - `backend/README.md`'s markdown-link line also shows 26.

  **Verified**: grep exit 1 (no match); `backend/README.md:32` reads "Node.js ... 26".
- [x] 8.7 Inside the shell (`nix develop`), run `npm rebuild`, then start the v0.10.x server with `npm run dev:server`. Verify that `better-sqlite3` loads without a `NODE_MODULE_VERSION` error under Node 26. **Verified**: `npm rebuild` succeeded; the server logged "Migrations completed successfully" and "Sidereal server running on port 5000" with no `NODE_MODULE_VERSION` error.
- [x] 8.8 Run `nix develop --command just check` and, outside the shell with a Node 26 version manager active, plain `just check`. Verify both exit with status 0. **Verified**: exit 0 inside (Node 26.10.0) and outside (native fnm, Node v26.2.0) — `just check` runs only the Rust checks, so the exact Node patch active outside Nix doesn't affect it.
- [x] 8.9 Run `openspec validate nix-dev-shell --strict`. Verify that it reports the change as valid. **Verified**: "Change 'nix-dev-shell' is valid".

## 9. Close the remaining CI and production-image node-version gaps

Section 8's task 8.5 and its own verification (8.6) checked only
`.github/workflows/ci.yml`. Three more workflow files also pin a Node
version, and both an earlier `/opsx:verify` pass and this task's own
tasks 8.5/8.6 missed them: they weren't in the grep's file list.
The maintainer then asked whether every artifact was actually
updated; a round-8 review, run in response, found that `Dockerfile`
pins Node 24 for the production image too, in both build stages --
also missed by every prior pass.

- [ ] 9.1 Change `node-version: '24'` to `'26'` in `.github/workflows/docker-build-test.yml`.
- [ ] 9.2 Change `node-version: '24'` to `'26'` in `.github/workflows/docker-build-push.yml`.
- [ ] 9.3 Change `node-version: '24'` to `'26'` in `.github/workflows/release.yml`.
- [ ] 9.4 Change `ci.yml`'s `node-version: '26.x'` to `'26'`, matching the plain-major-version format the other three files already use (pre-existing inconsistency, not introduced by this change; harmonizing since all four are being touched anyway). Verify every workflow's `node-version` agrees: `grep -rn node-version .github/workflows/` shows major version 26 on every matching line, with no `.x` suffix anywhere.
- [ ] 9.5 Confirm `backend-rs.yml` and `prune-ghcr.yml` have no `node-version` line, so this list is exhaustive: `grep -Lr node-version .github/workflows/*.yml` should include both.
- [ ] 9.6 Change both `FROM node:24-alpine` lines in `Dockerfile` (the `builder` and `runtime` stages) to `FROM node:26-alpine`. Verify with `grep -n 'FROM node:' Dockerfile`.
- [ ] 9.7 Build the production image locally (`docker build -t sidereal-test .`) and confirm it completes without error under Node 26. A successful build only proves dependencies compiled — it does not prove the app boots on Node 26's Alpine runtime, where `better-sqlite3` is compiled fresh against musl libc. Also run the image (`docker run -d -p 5000:5000 sidereal-test`) and confirm `curl -f http://localhost:5000/api/health` succeeds, matching the image's own `HEALTHCHECK`. Stop and remove the container afterward.
- [ ] 9.8 Run `openspec validate nix-dev-shell --strict`. Verify that it reports the change as valid.
