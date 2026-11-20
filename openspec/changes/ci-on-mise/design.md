## Context

`.github/workflows/ci.yml` has six jobs. Five of them install Nix with `DeterminateSystems/determinate-nix-action` and run every command as `nix develop --command <command>`. The `compose` job installs nothing and runs `sh scripts/compose-config-check.sh` directly, using the runner's Docker and jq.

The `mise` workflow (`.github/workflows/mise.yml`) already installs from `mise.lock` with `jdx/mise-action@v5`, `install_args: --locked` and `cache: false`, then runs tools as `mise exec -- <command>`. It proves that a GitHub runner can install every tool in `mise.toml` and run them without `mise activate`. [ADR-016](../../../docs/decisions/ADR-016-development-environment-with-mise.md) already decided that CI installs from the same lock as contributors.

Three facts shape the approach:

- `mise exec` does not run the `enter` hook in `mise.toml`, so no job runs `lets sync` by accident. The `lets-spec` job relies on this: it must compare the committed files, not rewrite them first.
- Rust is the `stable` channel in `mise.lock`, so rustup picks the release when the job runs. Rust is not reproducible from the lock alone, as ADR-016 accepts.
- The current spec's "The server job has one check step" scenario is false today: the `server` job has three `run` steps (fixture up, gate, fixture down). The new scenario keeps only the line that runs a `check-` recipe.

## Goals / Non-Goals

**Goals:**

- Every `ci.yml` job installs from `mise.lock` with one identical install step, and has no Nix step.
- Every command that needs a pinned tool runs as `mise exec -- <command>`.
- The jobs keep their names, their order of steps and what they check, so a reviewer sees the same checks pass or fail as before.

**Non-Goals:**

- Renaming `just` recipes to mise tasks. #499 does that.
- Changing the `nix` or `mise` workflows. #502 removes the `nix` workflow.
- Fixing the `ci` spec's path-filter requirements. That drift is a separate bug.
- Rewriting the contributor docs. Only the sentences about where CI gets Rust change (D9).

## Test Seams

The change is to a workflow file and docs, so there are no unit tests. Two seams prove it:

- **Primary test seam:** the `ci` spec's `yq` and `grep` scenarios, run against `.github/workflows/ci.yml` on the branch. They prove the shape of each job.
- **Existing seams reused:** the `ci` workflow's own run on this pull request. Every job passing proves the tools install and the checks run. `just check` locally proves the `openspec` gate accepts the change.

The "A stale lock fails the job" scenario is checked once, by hand, on a throwaway commit that adds a tool to `mise.toml` without updating `mise.lock`. That commit is not pushed to the pull request branch.

## Scrutiny Tier & Blast Radius

- **Tier:** medium
- **Blast radius:** every job in `.github/workflows/ci.yml`, and therefore every pull request's required checks. No application code, data or public contract changes. A mistake blocks merges until fixed, and is undone by reverting the workflow file.

## Decisions

### D-1: One install step, the same in every job

Each job gets this step straight after `actions/checkout`:

```yaml
- name: Install tools from mise.lock
  uses: jdx/mise-action@v5
  with:
    install_args: --locked
```

It installs every tool `mise.toml` names, in every job.

- **Alternative:** install only the tools each job needs, such as `--locked just` for `compose`. Rejected: `mise exec` installs any missing tool on its first run unless that is turned off, so a subset needs extra settings in every job. Each job's tool list would also have to change whenever a recipe starts using a new tool.
- **Cost:** the `compose` and `lets-spec` jobs install Rust and Node they do not use. That adds time but not risk.

`--locked` makes the install fail when `mise.lock` lacks an entry, rather than resolving a new release. This is what "installs only the releases `mise.lock` pins" means.

### D-2: Keep mise-action's cache on in `ci.yml`

`ci.yml` leaves `cache` at its default (on). The cache key includes the hash of `mise.lock`, so a lock change gets a fresh install.

- **Alternative:** `cache: false`, as in the `mise` workflow. Rejected for `ci.yml`: that workflow's job is to verify checksums, so it must download every time. `ci.yml` runs on every pull request in six jobs, and the `mise` workflow already verifies each lock change's checksums.
- **Trade-off:** a restored install skips the checksum check. See Risks.

Rust is not in mise-action's cache: rustup keeps it in its own directory. Each job that runs `rustc` or `cargo` downloads the stable toolchain, as the runner's rustup does today.

### D-3: Cache cargo output through `mise exec`

`Swatinem/rust-cache@v2` stays after the install step, with `cmd-format: mise exec -- {0}`. It must come after `jdx/mise-action`, because it runs `cargo` and `rustc` to build its cache key.

### D-4: The `compose` job runs its recipe

The `compose` job's run step becomes `mise exec -- just check-compose`. The recipe calls the same script, so the check does not change. Docker and jq still come from the runner (D4).

### D-5: The `e2e` and `lets-spec` jobs keep their steps

Each `nix develop --command` becomes `mise exec --`. The steps keep their order and their comments, which already give the reasons for the exemptions (D5, D6). The `lets-spec` comment changes to say the job checks against the release `mise.toml` names, and that `mise exec` runs no `enter` hook.

`pnpm exec playwright install --with-deps chromium` still runs in its own step. Playwright calls `sudo` to install the system libraries, which the runner allows.

### D-6: Remove and add requirements instead of modifying one

The spec delta removes "Every CI job runs one recipe inside the development shell" and adds five shorter requirements. OpenSpec 2 refuses a MODIFIED block that drops a scenario's name, and the old scenario names mention the shell. It also warns when a requirement's text is over 500 characters, which the combined rule would be.

### D-7: Correct three doc passages about CI's Rust

- `AGENTS.md`, the "Rust follows the latest stable release" bullet: `flake.lock` decides the release in the Nix shell and the `nix` workflow; in the `ci` workflow and on the mise route, rustup decides it.
- `CONTRIBUTING.md`, the "Without Nix" step 1 note: CI installs Rust's stable channel when each job runs, so it uses the latest stable release, as a contributor does after `rustup update`. The note no longer points to the `nix` flake check run.
- `CONTRIBUTING.md`, the Rust row of the versions table: `flake.lock` decides the release in the Nix shell and the `nix` workflow. In the `ci` workflow and with mise, `mise.lock` records the channel and rustup decides the release.

## Risks / Trade-offs

- [A new Rust stable release breaks a check, such as a new clippy lint] → This was already true for contributors with rustup. The fix is a normal pull request that fixes the warning. No lock change is needed, and none would help.
- [A restored mise-action cache skips the checksum check] → The cache is written only by runs of this repository's workflows, and the key changes with `mise.lock`. The `mise` workflow downloads with no cache and verifies checksums on every pull request that changes the lock.
- [Installing every tool makes the short jobs slower] → The cache keeps repeat installs short. If the time matters later, a job can pass a tool list to `install_args`, with mise's automatic install turned off.
- [Each job downloads Rust, because rustup's directory is not cached] → The download is about the size the runner's preinstalled rustup update fetches today. `Swatinem/rust-cache` still caches the build output, which is the slow part.
- [`grep -i 'nix'` matches a harmless word in a comment] → Comments in `ci.yml` avoid the word. The scenario is a quick check that no Nix step is left.

## Migration & Rollback

N/A for data. Rollback is reverting the commit that changes `.github/workflows/ci.yml`: the Nix steps come back, and `flake.lock` still pins the tools until #502 removes it. Branch protection names jobs, not steps, and the job names do not change, so no settings change.
