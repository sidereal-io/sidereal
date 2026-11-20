## REMOVED Requirements

### Requirement: Every CI job runs one recipe inside the development shell

**Reason**: CI no longer uses the Nix development shell. Five new requirements replace this one: "Every CI job installs its tools from mise.lock", "Every CI job runs its tool commands through mise", "Every CI job runs its checks as one recipe", "The lets-spec job checks the Let's Spec files" and "The e2e job builds the server before the end-to-end gate".

**Migration**: None for contributors. A reviewer who checked a job's steps for `DeterminateSystems/determinate-nix-action` or `nix develop` now checks them for `jdx/mise-action` and `mise exec --`.

## ADDED Requirements

### Requirement: Every CI job installs its tools from mise.lock

Every job in `.github/workflows/ci.yml` SHALL install the tools its checks use from `mise.lock`, and SHALL have no Nix step. It SHALL install only the releases that `mise.lock` pins: when `mise.lock` does not match `mise.toml`, the job fails rather than resolving a release of its own. No other step SHALL install a tool that `mise.toml` names. The GitHub runner provides Docker and jq.

#### Scenario: No job uses Nix

- **WHEN** a reviewer runs `grep -n -i 'nix' .github/workflows/ci.yml`
- **THEN** the search finds no match

#### Scenario: The server job installs its tools only through mise

- **WHEN** a reviewer runs `yq '.jobs.server.steps[] | select(has("uses")) | .uses | sub("@.*", "")' .github/workflows/ci.yml`
- **THEN** the output is exactly three lines, in this order: `actions/checkout`, `jdx/mise-action`, and `Swatinem/rust-cache`

#### Scenario: The web, openspec, compose and lets-spec jobs install their tools only through mise

- **WHEN** a reviewer runs `yq '.jobs.<job>.steps[] | select(has("uses")) | .uses | sub("@.*", "")' .github/workflows/ci.yml` for each of `web`, `openspec`, `compose` and `lets-spec`
- **THEN** each output is exactly two lines, in this order: `actions/checkout` and `jdx/mise-action`

#### Scenario: The e2e job installs its tools only through mise

- **WHEN** a reviewer runs `yq '.jobs.e2e.steps[] | select(has("uses")) | .uses | sub("@.*", "")' .github/workflows/ci.yml`
- **THEN** the output is exactly four lines, in this order: `actions/checkout`, `jdx/mise-action`, `Swatinem/rust-cache`, and `actions/upload-artifact`

#### Scenario: A stale lock fails the job

- **WHEN** a pull request adds a tool to `mise.toml` and does not change `mise.lock`
- **THEN** the `server` job fails on that pull request, in its mise install step

### Requirement: Every CI job runs its tool commands through mise

Every step in `.github/workflows/ci.yml` SHALL run each command that needs a tool from `mise.lock` as `mise exec -- <command>`. A step that caches cargo's build output SHALL run its own Rust commands through mise, so it installs no Rust of its own.

#### Scenario: Every tool command runs through mise

- **WHEN** a reviewer runs `yq '.jobs[].steps[] | select(has("run")) | .run' .github/workflows/ci.yml | grep -nE '(^|[ (])(cargo|pnpm|just|lets|openspec) ' | grep -v 'mise exec -- '`
- **THEN** the search finds no match

#### Scenario: The cache step uses mise's Rust

- **WHEN** a reviewer runs `yq '.jobs[].steps[] | select(.uses == "Swatinem/rust-cache*") | .with.cmd-format' .github/workflows/ci.yml`
- **THEN** the output is at least one line
- **AND** every line of the output is `mise exec -- {0}`

### Requirement: Every CI job runs its checks as one recipe

Every job in `.github/workflows/ci.yml` SHALL run its checks as one `just` recipe, and SHALL run no check that the recipe does not run. The `lets-spec` and `e2e` jobs are exempt from this requirement only. They still install their tools from `mise.lock` and run them through mise.

#### Scenario: A check in the recipe fails

- **WHEN** a pull request changes a file under `server/crates/` so that `cargo fmt --check` reports a difference
- **THEN** the `server` job fails on that pull request

### Requirement: The lets-spec job checks the Let's Spec files

The `lets-spec` job SHALL run `mise exec -- lets sync --check` instead of a `just` recipe. It checks the files that Let's Spec writes, not the project's code, and `just check` does not run it.

#### Scenario: The lets-spec job runs one check step

- **WHEN** a reviewer runs `yq '.jobs.lets-spec.steps[] | select(has("run")) | .run' .github/workflows/ci.yml`
- **THEN** the output is exactly one line: `mise exec -- lets sync --check`

#### Scenario: A Let's Spec file differs from the release

- **WHEN** a pull request changes a file under `.agents/skills/lets-plan/`
- **THEN** the `lets-spec` job fails on that pull request

### Requirement: The e2e job builds the server before the end-to-end gate

The `e2e` job SHALL run these steps in order: start the database fixture, build the server, install Chromium with its system libraries, run `just e2e`, and stop the fixture. Building first keeps compiling out of Playwright's 300-second startup window. Chromium's system libraries need root, which `just e2e` does not have, so they install in their own step.

#### Scenario: The server is built before the browser and the gate

- **WHEN** a reviewer runs `yq '.jobs.e2e.steps[] | select(has("run")) | .run' .github/workflows/ci.yml`
- **THEN** a line containing `mise exec -- cargo build -p sidereal-server` comes before a line containing `mise exec -- pnpm exec playwright install --with-deps chromium`
- **AND** that line comes before the line `mise exec -- just e2e`

## MODIFIED Requirements

### Requirement: Each CI job runs its own check recipe

The `compose` job's recipe is `check-compose`. The `server` job's recipe is `check-server`. The `web` job's recipe is `check-web`. The `openspec` job's recipe is `check-openspec`. Each job SHALL run its recipe as `mise exec -- just <recipe>`. `just check` SHALL run `check-compose`, then `check-openspec`, then `check-server`, then `check-web`, so a contributor and CI run the same gates.

#### Scenario: The compose job has one check step

- **WHEN** a reviewer runs `yq '.jobs.compose.steps[] | select(has("run")) | .run' .github/workflows/ci.yml`
- **THEN** the output is exactly one line: `mise exec -- just check-compose`

#### Scenario: The server job has one check step

- **WHEN** a reviewer runs `yq '.jobs.server.steps[] | select(has("run")) | .run' .github/workflows/ci.yml | grep 'just check-'`
- **THEN** the output is exactly one line: `mise exec -- just check-server`

#### Scenario: The web job has one check step

- **WHEN** a reviewer runs `yq '.jobs.web.steps[] | select(has("run")) | .run' .github/workflows/ci.yml`
- **THEN** the output is exactly one line: `mise exec -- just check-web`

#### Scenario: The openspec job has one check step

- **WHEN** a reviewer runs `yq '.jobs.openspec.steps[] | select(has("run")) | .run' .github/workflows/ci.yml`
- **THEN** the output is exactly one line: `mise exec -- just check-openspec`

#### Scenario: The local gate runs the same recipe

- **WHEN** a contributor runs `just --show check`
- **THEN** the output contains the line `check: check-compose check-openspec check-server check-web`
