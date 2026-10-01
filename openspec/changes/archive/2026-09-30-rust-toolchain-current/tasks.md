## 1. Toolchain

- [x] 1.1 Set `channel = "stable"` in `server/rust-toolchain.toml`, keeping the `rustfmt` and `clippy` components. Verify: `grep -n '^channel' server/rust-toolchain.toml` prints one line, `channel = "stable"`, and `nix develop --command rustc --version` reports 1.98.1.
- [x] 1.2 Rewrite the comment in `nix/toolchains.nix` so it no longer calls the file the only source of the Rust version: the file names the channel, and `flake.lock` decides the release. Verify: `nix flake check` exits with status 0.
- [x] 1.3 Remove `rust-version` from `server/Cargo.toml`. Verify: `grep -rn 'rust-version' server --include=Cargo.toml` finds nothing, and `nix develop --command just check-server` exits with status 0.

## 2. Server image

- [x] 2.1 In `server/Dockerfile`, build from `rust:1-slim-bookworm`, run `rustup toolchain install && cargo build --release -p sidereal-server` in one `RUN` step after `COPY . .`, and correct the comment that says "pinned 1.85". Verify: `grep -n '^FROM rust:' server/Dockerfile` names only `rust:1-` tags, and `docker build --progress=plain server/` exits with status 0 and prints no `auto-install` warning.

## 3. Flake check

- [x] 3.1 Add `rustc --version` to the "Tool versions" step in `.github/workflows/nix.yml`, inside the same `nix develop --command` call. Verify: the step's `run` line contains `rustc --version`, and the flake check log on this pull request shows `rustc 1.98.1`.

## 4. Docs

- [x] 4.1 Rewrite the "Toolchain is pinned" rule in `AGENTS.md`: Rust follows the latest stable release, `server/rust-toolchain.toml` names the `stable` channel, and `flake.lock` decides the exact release in the Nix shell and CI. Keep the `just check` gate sentence. Verify: the docs search in 4.4 finds no match in `AGENTS.md`.
- [x] 4.2 Remove both mentions of 1.85 from `server/README.md`: the crate-layout comment and the rustup prerequisite. Verify: the docs search in 4.4 finds no match in `server/README.md`.
- [x] 4.3 Update `CONTRIBUTING.md` as design D5 describes: "With Nix" step 4, "Without Nix" step 1, and a new bullet in "Reviewing a pin update". Verify: each of the three places says what D5 says, and none claims that rustup matches CI.
- [x] 4.4 Run the docs search from the spec. Verify: `grep -nE '\b1\.[0-9]{2}(\.[0-9]+)?\b' AGENTS.md README.md server/README.md CONTRIBUTING.md` finds no match.

## 5. Whole-change checks

- [x] 5.1 Run the full gate in the shell. Verify: `nix develop --command just check` exits with status 0.
- [x] 5.2 Check that the shell reloads on a channel change. Set `channel = "1.97.1"`, run `direnv export bash`, and check `rustc --version`; then restore `stable`. Verify: the shell reports 1.97.1 after the edit, and `git diff server/rust-toolchain.toml` is empty after the restore.
- [x] 5.3 Push and check CI. Verify: the `nix` flake check and the `v2` server job both pass on the pull request.
- [x] 5.4 Validate the change. Verify: `openspec validate rust-toolchain-current --strict` reports the change as valid.
