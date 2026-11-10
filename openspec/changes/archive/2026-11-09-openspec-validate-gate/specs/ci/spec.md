## ADDED Requirements

### Requirement: The OpenSpec checks run on every pull request

A CI job named `openspec` SHALL run the OpenSpec checks. The job lives in the workflow file `.github/workflows/ci.yml`. It SHALL run on every pull request, and on every push to `main`, that triggers that workflow.

The checks are those of `just check-openspec`: strict validation of every spec and every active change. The job SHALL fail when validation reports an error or a warning.

#### Scenario: A pull request changes only a spec

- **WHEN** a pull request changes only a file under `openspec/specs/`
- **THEN** the `openspec` job runs on that pull request

#### Scenario: A spec warning fails the job

- **WHEN** a pull request shortens one spec's Purpose section under `openspec/specs/` to the word `Short.`
- **THEN** the `openspec` job fails on that pull request

#### Scenario: An invalid change fails the job

- **WHEN** a pull request adds a requirement with no scenario to a delta spec in an active change under `openspec/changes/`
- **THEN** the `openspec` job fails on that pull request

## MODIFIED Requirements

### Requirement: Every CI job runs one recipe inside the development shell

Every job in `.github/workflows/ci.yml` SHALL meet these four rules:

- The job runs its checks as one `just` recipe inside the Nix development shell.
- The job runs no check that the recipe does not run.
- The job has no step that installs Rust, Node, pnpm, or any other tool the checks use. The shell provides them.
- A step that caches cargo's build output runs its own Rust commands inside the shell, so it installs no Rust either.

The `server` job's recipe is `check-server`. The `web` job's recipe is `check-web`. The `openspec` job's recipe is `check-openspec`. `just check` SHALL run `check-compose`, then `check-openspec`, then `check-server`, then `check-web`, so a contributor and CI run the same gates.

The scenarios below use `yq` version 4 (the Go implementation). The development shell does not provide it, so a reviewer installs it first.

#### Scenario: The server job has one check step

- **WHEN** a reviewer runs `yq '.jobs.server.steps[] | select(has("run")) | .run' .github/workflows/ci.yml`
- **THEN** the output is exactly one line: `nix develop --command just check-server`

#### Scenario: The server job has no step that installs a tool

- **WHEN** a reviewer runs `yq '.jobs.server.steps[] | select(has("uses")) | .uses | sub("@.*", "")' .github/workflows/ci.yml`
- **THEN** the output is exactly three lines, in this order: `actions/checkout`, `DeterminateSystems/determinate-nix-action`, and `Swatinem/rust-cache`

#### Scenario: The cache step uses the shell's Rust

- **WHEN** a reviewer runs `yq '.jobs.server.steps[] | select(.uses == "Swatinem/rust-cache*") | .with.cmd-format' .github/workflows/ci.yml`
- **THEN** the output is exactly one line: `nix develop -c {0}`

#### Scenario: The web job has one check step

- **WHEN** a reviewer runs `yq '.jobs.web.steps[] | select(has("run")) | .run' .github/workflows/ci.yml`
- **THEN** the output is exactly one line: `nix develop --command just check-web`

#### Scenario: The web job has no step that installs a tool

- **WHEN** a reviewer runs `yq '.jobs.web.steps[] | select(has("uses")) | .uses | sub("@.*", "")' .github/workflows/ci.yml`
- **THEN** the output is exactly two lines, in this order: `actions/checkout` and `DeterminateSystems/determinate-nix-action`

#### Scenario: The openspec job has one check step

- **WHEN** a reviewer runs `yq '.jobs.openspec.steps[] | select(has("run")) | .run' .github/workflows/ci.yml`
- **THEN** the output is exactly one line: `nix develop --command just check-openspec`

#### Scenario: The openspec job has no step that installs a tool

- **WHEN** a reviewer runs `yq '.jobs.openspec.steps[] | select(has("uses")) | .uses | sub("@.*", "")' .github/workflows/ci.yml`
- **THEN** the output is exactly two lines, in this order: `actions/checkout` and `DeterminateSystems/determinate-nix-action`

#### Scenario: The local gate runs the same recipe

- **WHEN** a contributor runs `just --show check`
- **THEN** the output contains the line `check: check-compose check-openspec check-server check-web`

#### Scenario: A check in the recipe fails

- **WHEN** a pull request changes a file under `server/crates/` so that `cargo fmt --check` reports a difference
- **THEN** the `server` job fails on that pull request
