## 1. Move the workspace (one commit, design D1)

- [x] 1.1 Run `git mv backend server`, and verify `git status` shows only renames under `server/`.
- [x] 1.2 Update the `justfile`: point every cargo path and the arch lint at `server/`, rename the `backend` recipe to `server`, and fix the header comments. Verify `just --list` shows `server` and no `backend`.
- [x] 1.3 Update `.envrc` (the watched pin file and its comment), `nix/toolchains.nix` (the pin file path), and `.gitignore` (`server/target/`). Verify `nix flake check` passes and `direnv status` lists `server/rust-toolchain.toml` as watched.
- [x] 1.4 Update the path in comments in `server/Dockerfile`, `server/crates/packs/astro/Cargo.toml`, and `server/crates/packs/astro/src/lib.rs`. Leave the Dockerfile's base image and version text to #310. Verify `just check` passes, then commit the move and these edits together.

## 2. CI workflows and Dependabot

- [x] 2.1 Run `git mv .github/workflows/backend-rs.yml .github/workflows/v2.yml`. Set the workflow `name` to `v2` and the job id to `server`. Set the triggers to `server/**`, `justfile`, and `.github/workflows/v2.yml`. Set the working directory and the rust-cache `workspaces` to `server`. Keep every step. Verify `yq '.jobs | keys' .github/workflows/v2.yml` prints only `server`.
- [x] 2.2 Change the `backend/rust-toolchain.toml` trigger in `.github/workflows/nix.yml` to `server/rust-toolchain.toml`. Verify `grep -rn backend .github/workflows/` finds no match.
- [x] 2.3 Create `.github/workflows/codeql.yml` with the `codeql` job from `ci.yml`, unchanged, and the same `push` and `pull_request` triggers, with no path filter. Remove the job from `ci.yml`. Verify `yq '.on' .github/workflows/codeql.yml` shows no `paths` or `paths-ignore` key, and `grep -rlE 'codeql-action/(init|analyze)' .github/workflows/` lists only `codeql.yml`. (Other workflows use `codeql-action/upload-sarif` for Trivy results; that is not a CodeQL job.)
- [x] 2.4 Add `paths-ignore: [server/**, web/**, openspec/**, docs/**]` to both the `push` and `pull_request` triggers of `ci.yml` and `docker-build-push.yml`. Verify with `yq '.on' <file>` for each file.
- [x] 2.5 Change the `cargo` entry's `directory` in `.github/dependabot.yml` to `/server`, and its comment if it names the path. Verify the spec's `yq '.updates[] | select(."package-ecosystem" == "cargo")'` scenario shows `directory: /server`.
- [x] 2.6 Run `actionlint` on every changed workflow (`nix shell nixpkgs#actionlint` if it is not installed), and verify it reports no error. Commit the CI and Dependabot edits.

## 3. Contributor docs

- [x] 3.1 Update the path references in `AGENTS.md`: the stack table, the "Where to read more" link, the durable-constraints section, and the workflow section. Change paths only; the Rust version wording belongs to #310. Verify `grep -n 'backend/' AGENTS.md` finds no match.
- [x] 3.2 Update `CONTRIBUTING.md`: the `target` rebuild note, the rustup pin-file path, and the `backend-rs` workflow name at the Cargo pull request note. Add one line telling contributors to run `rm -rf backend/target` after pulling the rename. Verify `grep -n 'backend' CONTRIBUTING.md` shows only the new cleanup line and v0.10.x's own "backend server" wording.
- [x] 3.3 Update `server/README.md`: the layout block, the cargo commands, and `just server`. Verify `grep -n 'backend/\|just backend' server/README.md` finds no match.
- [x] 3.4 Update `openspec/discovery.md`: the Work & check and Open PR stages no longer list the `backend/` location or v0.10.x CI on v2-only pull requests as gaps, and the Bump pins stage names `server/Cargo.lock`. Verify `grep -n 'backend/' openspec/discovery.md` finds no match. Commit the docs.

## 4. Verify the whole change

- [ ] 4.1 Run `git grep -n 'backend/' -- . ':!openspec/changes/archive' ':!apps' ':!packages' ':!CHANGELOG.md' ':!package-lock.json'`, and verify the only matches are this change's own artifacts and the new cleanup line in `CONTRIBUTING.md`.
- [ ] 4.2 Run `just check` and `nix develop --command just check`, and verify both exit with status 0.
- [ ] 4.3 Run `openspec validate server-dir-rename-and-ci-split --strict`, and verify it passes.
- [ ] 4.4 On the pull request, verify that the `v2 / server`, `nix`, `CodeQL`, and `ci.yml` checks all run and pass. This pull request changes root files, so every workflow runs. Record in the pull request description that the first v2-only pull request must show `ci.yml` and `docker-build-push.yml` skipped and CodeQL still running.
