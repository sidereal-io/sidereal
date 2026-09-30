## Context

See proposal.md for why. This section covers only the current state that shapes the approach.

- The `server` job in `v2.yml` runs seven steps after checkout: `rustup show`, `Swatinem/rust-cache`, and five check steps. Its default working directory is `server`.
- The `check` recipe in the `justfile` runs four of those five checks. It does not run `cargo build`.
- `nix.yml` already installs Nix with `DeterminateSystems/determinate-nix-action@v3`, then runs `nix flake check`. Installing Nix takes about 9 seconds, and building the shell takes about 40.
- The shell's hook runs `just enter` each time the shell loads. That recipe generates the OpenSpec skills when they are stale. It prints nothing on success and never fails the shell.
- `v2.yml` runs on `pull_request` only. GitHub lets a pull request restore a cache saved on its own branch or on `main`. No run saves a cache on `main` today.
- The `sidereal-server` crate has an integration test, `tests/healthz.rs`.

## Goals / Non-Goals

**Goals:**

- One recipe defines the server gate. CI and contributors both run it.
- The checks run with the Rust that the shell pins. The job installs no other Rust.

**Non-Goals:**

- Sharing the Nix install step between `nix.yml` and `v2.yml`. Two copies of one step are simpler than a composite action.
- Making the job faster. #320 and #288 own the cache work.
- Changing what `just check` runs for a contributor.

## Decisions

### D1. Split the recipe: `check-server` holds the gate, and `check` depends on it

The body of today's `check` recipe moves, unchanged, into a new `check-server` recipe. `check` keeps its name and becomes `check: check-server`, with no body.

- **Why:** CI needs a recipe that runs the server gate only. #301 adds `check-web` as a second dependency of `check`, and a second CI job that runs it.
- **Alternative:** have CI run `just check`. Rejected, because after #301 the `server` job would also run the web gate.

### D2. Drop the separate `cargo build` check

The `server` job stops running `cargo build`. The `check-server` recipe does not gain it.

- **Why:** the other checks already compile everything `cargo build` does. `cargo clippy --all-targets` type-checks every target. `cargo test` compiles every target, and it links the `sidereal-server` binary for the integration test.
- **Evidence:** the build step takes 1 second in CI, because nothing is left for it to compile.
- **Alternative:** add `cargo build` to the recipe. Rejected, because it adds time to every local `just check` and finds no error the other checks miss.
- The production image still runs `cargo build --release`. No CI job builds that image today, and this change does not add one.

### D3. Copy the Nix install step from `nix.yml`

The `server` job gains one step: `uses: DeterminateSystems/determinate-nix-action@v3`, with no inputs. It matches the step in `nix.yml`.

- **Why:** one installer across both workflows means one thing to update. Dependabot already tracks the action's version in each file.

### D4. One step runs the gate

After the cache step, the job has one `run` step: `nix develop --command just check-server`. The job's default working directory goes away, so the step runs at the repo root, where the `justfile` sits.

- **Why one step:** any check listed in the workflow is a check the recipe can drift from.
- **Cost:** the job log shows one step, not five. The log still names the failing command, because `just` prints each command before it runs it.
- The job prints no tool versions. `nix.yml` already does, on every pull request that changes the shell.

### D5. Point rust-cache at the shell with `cmd-format`

The `Swatinem/rust-cache@v2` step keeps `workspaces: server` and gains `cmd-format: nix develop -c {0}`. It moves to after the Nix install step.

- **Why:** rust-cache runs `rustc -vV` and `cargo metadata` to build its cache key. By default it runs the runner's own copies. In `server/`, the runner's rustup would download Rust 1.85.0 just to answer. `cmd-format` makes rust-cache run both commands inside the shell, so the job installs no second Rust. The rust-cache README documents this setting for Nix.
- **Limit:** rust-cache also asks rustup which toolchains the runner has, and adds each version to the cache key. The shell does not hide the runner's rustup, so the runner's stable Rust stays in the key. This was true before this change, and rust-cache has no setting to turn it off. The runner's Rust never compiles or checks the code.
- **Cost:** rust-cache loads the shell a few times, at one to three seconds each. Its first call also builds the shell, so that time moves from the check step to the cache step.
- **Alternative:** leave rust-cache's settings alone, as #289 suggests. Rejected, because a second Rust toolchain in the job is the drift this change removes. The cache key would still change on a Rust bump, because it includes a hash of `rust-toolchain.toml`.
- **Alternative:** drop rust-cache. Rejected, because #320 makes it useful: once `main` saves a cache, pull requests skip compiling the dependencies.

### D6. Run the job when the shell changes

The `pull_request` path filter gains `flake.nix`, `flake.lock`, and `nix/**`.

- **Why:** the shell now supplies every tool the gate uses. A pin update can change a lint result or break the build.
- **Cost:** a pull request that changes the shell runs both the flake check and the `server` job. Each builds the shell once.
- `.envrc` stays out of the filter. `nix develop` does not read it.

### D7. Put the rule in the `ci` spec

The delta changes the `ci` spec only. See proposal.md, Capabilities, for the reason.

## Risks / Trade-offs

- **[Trade-off] The job takes about three times as long.** It grows from about 20 seconds to about 70. → Accepted. The job still finishes well before the v0.10.x pipeline. #288 can cache the shell later.
- **[Risk] `cmd-format` behaves differently from the README.** rust-cache runs `cargo metadata` with `server/` as its working directory, and `nix develop` must find the flake from there. Nix searches parent directories up to the git root, so it should. → The first run of this pull request proves it. If it fails, the author stops and agrees a new D5 with the maintainer. Removing `cmd-format` alone is not a fallback: rustup would then install Rust 1.85.0, which the spec forbids.
- **[Trade-off] The cache key changes when GitHub updates the runner's Rust.** See D5, Limit. The next run then compiles from a cold cache, though the pinned Rust did not change. → Accepted. It costs one slow run about every six weeks, and the checks stay correct.
- **[Risk] A later target compiles but does not link, and no check notices.** D2 relies on `cargo test` linking every binary. A binary that opts out of tests would lose that cover. → No such target exists. Whoever adds one adds `cargo build` to `check-server` in the same change.
- **[Risk] The shell hook writes to standard output and corrupts what rust-cache reads.** → The hook prints nothing on success, and a failed refresh prints to standard error only. The `dev-environment` spec requires both.
- **[Risk] A pull request's checks depend on two outside download sites.** The shell's packages come from `cache.nixos.org`, and its Rust toolchain comes from `static.rust-lang.org`. → Today's job already depends on the second. A failed download fails the job visibly, and a rerun fixes it.
- **[Trade-off] Each run generates the OpenSpec skills.** The hook does this in the CI checkout, which has none. → Accepted: it takes about a second, needs no network, and cannot fail the job.
- **[Trade-off] Every pull request still compiles from a cold cache.** No run saves a cache on `main`. → #320 fixes this, as the next change.
- **[Trade-off] This pull request also runs the v0.10.x pipeline.** It changes the root `justfile`, which is outside that pipeline's ignore list. → #322 covers the ignore list.

## Migration Plan

1. Merge this pull request. Nothing deploys.
2. Contributors need no action. `just check` runs the same four checks as before.
3. Rollback: revert the merge commit.
