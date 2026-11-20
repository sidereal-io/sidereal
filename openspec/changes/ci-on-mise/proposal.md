## Why

The `ci` workflow installs its tools through Nix, while contributors now install theirs from `mise.lock` (#497). A pull request can therefore pass in CI with different tool releases from the ones its author ran. Moving CI onto `mise.lock` gives both one lock file, and it is the next step before the recipes move to mise tasks (#499) and the Nix route goes (#502).

## What Changes

- Every job in `.github/workflows/ci.yml` installs its tools from `mise.lock` and has no Nix step (D1, D2). The jobs are `compose`, `server`, `web`, `openspec`, `e2e` and `lets-spec`.
- Each job runs its commands as `mise exec -- <command>`. The `compose`, `server`, `web` and `openspec` jobs each run one recipe as `mise exec -- just <recipe>` (D2, D4).
- The `compose` job now runs `just check-compose` instead of calling the script directly. Docker and jq still come from the runner (D4).
- The cargo cache step runs its Rust commands through mise, so it installs no Rust of its own (D3).
- The `lets-spec` job runs `mise exec -- lets sync --check`. It is exempt from the one-recipe rule, but not from installing through mise (D5).
- The `e2e` job keeps its steps in their current order, each through mise. It is exempt from the one-recipe rule for the reasons already in `ci.yml` (D6).
- The `ci` spec says mise instead of the Nix shell, names the exempt jobs and why, and names `check-compose` as the `compose` job's recipe (D8).
- `AGENTS.md` and `CONTRIBUTING.md` stop saying the `ci` workflow takes its Rust release from `flake.lock`. Only those sentences change (D9).

Out of scope: renaming recipes to mise tasks (#499), the `nix` workflow (#502), and the `ci` spec's outdated path-filter requirements, which become a separate bug (D7).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `ci`: "Every CI job runs one recipe inside the development shell" installs tools from `mise.lock` instead of the Nix shell, and names `lets-spec` and `e2e` as exempt from the one-recipe rule. "Each CI job runs its own check recipe" adds `check-compose` for the `compose` job, and its scenarios expect `mise exec -- just <recipe>`.

## Impact

- `.github/workflows/ci.yml`: all six jobs.
- `openspec/specs/ci/spec.md`: two requirements and their scenarios.
- `AGENTS.md` and `CONTRIBUTING.md`: the sentences that say where CI gets Rust.
- CI run time: each job installs tools through `jdx/mise-action` instead of the Nix cache, so job times change.
- Nothing changes for contributors' local tools, `mise.toml`, `mise.lock`, the `justfile`, the `nix` workflow or the `mise` workflow.
