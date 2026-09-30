## MODIFIED Requirements

### Requirement: The v2 server checks run when v2 server code changes

A CI job named `server`, in the workflow file `.github/workflows/v2.yml`, SHALL run the v2 server checks on every pull request that changes one of these files:

- any file under `server/`;
- the root `justfile`;
- `flake.nix` or `flake.lock`;
- any file under `nix/`;
- `.github/workflows/v2.yml`.

The checks are the Rust format check, clippy with warnings denied, the tests, and the dependency-direction lint. The job SHALL fail when any check fails.

#### Scenario: A pull request changes Rust code

- **WHEN** a pull request changes a file under `server/crates/`
- **THEN** the `server` job runs on that pull request
- **AND** the job fails if `cargo clippy --all-targets -- -D warnings` reports a warning

#### Scenario: A pull request changes only the shell's pins

- **WHEN** a pull request changes `flake.lock` only
- **THEN** the `server` job runs on that pull request

#### Scenario: A pull request changes only v0.10.x code

- **WHEN** a pull request changes files under `apps/` only
- **THEN** GitHub does not run the `server` job on that pull request

#### Scenario: The old workflow is gone

- **WHEN** a reviewer lists `.github/workflows/`
- **THEN** `v2.yml` exists and `backend-rs.yml` does not

## ADDED Requirements

### Requirement: Every v2 job runs one recipe inside the development shell

Every job in `.github/workflows/v2.yml` SHALL run its checks as one `just` recipe inside the Nix development shell. The job SHALL run no check that the recipe does not run. The job SHALL NOT install Rust, or any other tool the checks use, through a separate step. The `server` job's recipe is `check-server`. `just check` SHALL run `check-server`, so a contributor and CI run the same server gate.

#### Scenario: The server job has one check step

- **WHEN** a reviewer runs `yq '.jobs.server.steps[] | select(has("run")) | .run' .github/workflows/v2.yml`
- **THEN** the output is exactly one line: `nix develop --command just check-server`

#### Scenario: The server job does not install Rust itself

- **WHEN** a reviewer runs `grep -n rustup .github/workflows/v2.yml`
- **THEN** the search finds no match

#### Scenario: The local gate runs the same recipe

- **WHEN** a contributor runs `just --show check`
- **THEN** the output contains the line `check: check-server`

#### Scenario: A check in the recipe fails

- **WHEN** a pull request changes a file under `server/crates/` so that `cargo fmt --check` reports a difference
- **THEN** the `server` job fails on that pull request
