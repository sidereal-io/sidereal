## Context

See proposal.md for why. This section covers only the current state that shapes the approach.

- The Rust workspace sits in `backend/`. Fifteen live files name that path, across build config, CI, Nix, and docs. Archived OpenSpec changes also name it, but they are history.
- `backend-rs.yml` runs the Rust checks on pull requests that change `backend/**`. It installs Rust through `rustup show` and caches cargo output with `Swatinem/rust-cache`.
- `ci.yml` holds two jobs: the v0.10.x pipeline (`ci-pipeline`) and CodeQL (`codeql`). Both run on every push and pull request to `main` and `v0.x`.
- `docker-build-push.yml` builds the v0.10.x image on every pull request, and pushes it on every push to `main`.
- The `Protect Default` ruleset targets `main`, but its enforcement is disabled, so no rule applies today. It holds a `code_scanning` rule for CodeQL and no required status checks.
- The Nix flake reads `backend/rust-toolchain.toml` through `nix/toolchains.nix`. Nix sees only files that git tracks.
- No CI workflow builds `backend/Dockerfile`, so its build context has no CI caller to update.

## Goals / Non-Goals

**Goals:**

- Every commit on the branch passes `just check`, so the history stays bisectable.
- `git log --follow` still traces each Rust file back through the rename.

**Non-Goals:**

- Changing any check the `server` job runs. #289 moves those checks into the Nix shell.
- Adding CodeQL for Rust, or changing CodeQL's configuration.
- Changing the Dockerfile's base image. #310 owns that line.

## Decisions

### D1. Move the directory and fix its path references in one commit

One commit runs `git mv backend server` and updates every file that must change for the build to work. These are the justfile, `.envrc`, `nix/toolchains.nix`, `.gitignore`, and the arch lint's own paths.

- **Why:** a commit that only moves the directory breaks `just check` and the Nix shell. Bisecting would then land on a broken commit.
- **Alternative:** move first, fix references next. Rejected, because the first commit would not build.
- Docs, Dependabot, and CI move in later commits. None of them affect whether the build works.

### D2. Rename paths only, not the concept

Only text that names the directory changes. The word "backend" as a concept stays, for example "the Rust backend" in ADRs, `docs/architecture.md`, and `openspec/migration.md`. Crate names stay too, including `sidereal-server`.

- **Why:** ADRs stand alone and record decisions, not folder names. Renaming the concept would touch many files for no change in meaning.
- **Test:** `git grep -n 'backend/'` finds no match outside `openspec/changes/archive/`, `apps/`, `packages/`, `CHANGELOG.md`, and `package-lock.json`.

### D3. Move CodeQL into its own workflow, with no path filter

A new `.github/workflows/codeql.yml` takes the `codeql` job from `ci.yml` unchanged. It keeps the same triggers: push and pull request to `main` and `v0.x`. It has no `paths` or `paths-ignore` key.

- **Why:** D4 adds `paths-ignore` to `ci.yml`, including `web/**`. If CodeQL stayed there, it would never scan the v2 web app. That app will be TypeScript, the language this CodeQL job analyzes.
- **Second reason:** the ruleset holds a `code_scanning` rule. If a maintainer turns enforcement on, a pull request with no CodeQL results cannot merge. An unfiltered workflow always produces results.
- **Cost:** CodeQL also runs on pull requests that change only Rust code. It scans JavaScript and TypeScript that did not change. The run time is accepted in exchange for the two reasons above.
- **Alternative:** give CodeQL its own path filter. Rejected, because a path filter would need updating each time a new TypeScript directory appears.
- The ruleset's rule names the CodeQL tool, not a workflow or job. But CodeQL files each analysis under a category, and the default category includes the workflow file's path. Moving the job changes the category from `.github/workflows/ci.yml:codeql` to `.github/workflows/codeql.yml:codeql`.
- **Pin the category.** The analyze step sets `category: /language:javascript-typescript`. A later rename of the workflow file then keeps the same category, so `main` keeps its baseline.

### D4. Skip the v0.10.x workflows with `paths-ignore`

`ci.yml` and `docker-build-push.yml` gain `paths-ignore: [server/**, web/**, openspec/**, docs/**]` on both their `push` and `pull_request` triggers.

- **Why an ignore list, not an allow list:** v0.10.x has many root files (`package.json`, `vite.config.ts`, `tsconfig.json`, `Dockerfile`, and more). An allow list would miss a new root file and skip checks it needs. An ignore list fails safe: any file outside the four directories still runs the pipeline.
- **Why also on `push`:** a v2-only merge to `main` then builds and pushes no new v0.10.x image. The image would be identical anyway.
- `web/**` does not exist yet. Listing it now means #300 needs no workflow edit.
- `docker-build-test.yml` already has an allow list of v0.10.x paths, so it needs no change.

### D5. Rename `backend-rs.yml` to `v2.yml`, and keep its steps

`git mv` renames the file. The workflow's `name` becomes `v2`, and its one job becomes `server`. The trigger paths become `server/**`, `justfile`, and `.github/workflows/v2.yml`. The working directory and the rust-cache `workspaces` setting become `server`. The steps stay the same.

- **Why add `justfile` to the trigger:** the justfile defines the local gate. A change to it must prove that CI still passes.
- **Why keep the steps:** #289 replaces them with `nix develop --command just check-server`. Changing them twice would waste review effort.

### D6. Put the CI routing rules in a new `ci` capability

The rules about which workflow runs for which change go in a new `ci` spec. `dev-environment` keeps its existing flake-check rules, with only their paths updated.

- **Why:** `dev-environment` is about the pinned toolchain. Workflow routing is a separate concern that #289, #301, and #303 will extend.

## Risks / Trade-offs

- **[Risk] Open pull requests and branches that touch `backend/` go stale.** → Only Dependabot PR #318 is open, and it changes `flake.lock` only. Git follows the rename on rebase for most edits.
- **[Risk] A Dependabot Cargo pull request opened before the merge targets `/backend`.** → None is open today. If one is, a maintainer closes it after the merge. Dependabot's next weekly run opens a new one against `/server`.
- **[Risk] A skipped workflow never reports a status.** If `ci.yml` or `docker-build-push.yml` became a required status check, a v2-only pull request would wait forever. → No check is required today: the ruleset has no required-status-check rule, and `main` has no branch protection. If a maintainer adds one, they add an always-run aggregator job, as the epic's shared decisions already state.
- **[Risk] Contributors keep a stale `backend/target/` directory.** → Git ignores it, so it causes no errors. `CONTRIBUTING.md` tells contributors to delete it after pulling.
- **[Risk] The Nix shell misses the new pin file.** → Nix reads only files that git tracks. `git mv` stages the move, and `nix flake check` on this pull request proves the shell still builds.
- **[Trade-off] This pull request cannot prove the path filters.** It changes root files, so every workflow runs on it. → The next v2-only pull request, likely #289, shows `ci.yml` and `docker-build-push.yml` skipped, and CodeQL still reporting.
- **[Risk] This pull request has no CodeQL baseline.** `main` has no analysis under the new category, so GitHub skips the CodeQL result check on this pull request. The `codeql` job itself still runs and passes. → This happens once. The first push to `main` after the merge creates the baseline. The ruleset is disabled, so the skipped check blocks nothing.
- **[Risk] The old category stays on `main` as a stale setup, and its alerts stay open.** → Migration step 3 deletes it.
- **[Risk] #310 lands first and edits the same lines.** → Whichever story merges second rebases and updates the other's paths. Both issue bodies already say so.

## Migration Plan

1. Merge this pull request. Nothing deploys, because v2 has no running service.
2. Each contributor pulls, then runs `rm -rf backend/target`. With direnv, the shell reloads on its own, because `.envrc` now watches `server/rust-toolchain.toml`.
3. After the merge, a maintainer deletes the stale CodeQL setup: open Security → Code scanning → Tool status → CodeQL, then delete the `.github/workflows/ci.yml:codeql` setup. This closes alerts that only the old category holds.
4. Rollback: revert the merge commit. The rename is a pure move, so a revert restores every path.
