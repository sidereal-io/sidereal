# ci Specification

## Purpose

CI runs the checks that match what a pull request or a push to `main` changes, and code scanning runs on every pull request. `main` builds and releases nothing, and a weekly scan watches the `v0.x` maintenance branch for known vulnerabilities.

## Requirements

### Requirement: The v2 server checks run when v2 server code changes

A CI job named `server`, in the workflow file `.github/workflows/v2.yml`, SHALL run the v2 server checks on every pull request that changes one of these files:

- any file under `server/`;
- any file under `web/`;
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

#### Scenario: A pull request changes only web code

- **WHEN** a pull request changes files under `web/` only
- **THEN** the `server` job runs on that pull request

#### Scenario: A pull request changes only v0.10.x code

- **WHEN** a pull request changes files under `apps/` only
- **THEN** GitHub does not run the `server` job on that pull request

#### Scenario: The old workflow is gone

- **WHEN** a reviewer lists `.github/workflows/`
- **THEN** `v2.yml` exists and `backend-rs.yml` does not

### Requirement: The v0.10.x pipeline skips v2-only changes

The v0.10.x workflows `ci.yml` and `docker-build-push.yml` SHALL NOT run for a pull request or a push whose changed files all sit under `server/`, `web/`, `openspec/`, or `docs/`. They SHALL run when at least one changed file sits outside those four directories.

#### Scenario: A pull request changes only v2 code and plans

- **WHEN** a pull request changes files under `server/` and `openspec/` only
- **THEN** GitHub runs neither `ci.yml` nor `docker-build-push.yml` on that pull request

#### Scenario: A pull request changes v2 code and a root file

- **WHEN** a pull request changes a file under `server/` and also changes `package.json`
- **THEN** GitHub runs both `ci.yml` and `docker-build-push.yml` on that pull request

#### Scenario: A v2-only merge reaches main

- **WHEN** a commit that changes files under `server/` only is pushed to `main`
- **THEN** `docker-build-push.yml` does not run, and no new v0.10.x image is pushed

### Requirement: Code scanning runs on every pull request

CodeQL SHALL analyze the repo on every pull request to `main` or `v0.x`, and on every push to those branches, whatever files the change touches. The CodeQL workflow SHALL have no path filter. Every pull request therefore has CodeQL results, including one that changes only v2 code.

#### Scenario: A v2-only pull request gets code scanning results

- **WHEN** a pull request to `main` changes files under `server/` only
- **THEN** the CodeQL workflow runs on that pull request
- **AND** the pull request's checks list a completed CodeQL analysis

#### Scenario: The CodeQL workflow has no path filter

- **WHEN** a reviewer reads `.github/workflows/codeql.yml`
- **THEN** its `on` block contains no `paths` or `paths-ignore` key
- **AND** no other workflow file defines a CodeQL job

### Requirement: Every v2 job runs one recipe inside the development shell

Every job in `.github/workflows/v2.yml` SHALL meet these four rules:

- The job runs its checks as one `just` recipe inside the Nix development shell.
- The job runs no check that the recipe does not run.
- The job has no step that installs Rust, Node, pnpm, or any other tool the checks use. The shell provides them.
- A step that caches cargo's build output runs its own Rust commands inside the shell, so it installs no Rust either.

The `server` job's recipe is `check-server`. The `web` job's recipe is `check-web`. `just check` SHALL run `check-server`, then `check-web`, so a contributor and CI run the same gates.

The scenarios below use `yq` version 4 (the Go implementation). The development shell does not provide it, so a reviewer installs it first.

#### Scenario: The server job has one check step

- **WHEN** a reviewer runs `yq '.jobs.server.steps[] | select(has("run")) | .run' .github/workflows/v2.yml`
- **THEN** the output is exactly one line: `nix develop --command just check-server`

#### Scenario: The server job has no step that installs a tool

- **WHEN** a reviewer runs `yq '.jobs.server.steps[] | select(has("uses")) | .uses | sub("@.*", "")' .github/workflows/v2.yml`
- **THEN** the output is exactly three lines, in this order: `actions/checkout`, `DeterminateSystems/determinate-nix-action`, and `Swatinem/rust-cache`

#### Scenario: The cache step uses the shell's Rust

- **WHEN** a reviewer runs `yq '.jobs.server.steps[] | select(.uses == "Swatinem/rust-cache*") | .with.cmd-format' .github/workflows/v2.yml`
- **THEN** the output is exactly one line: `nix develop -c {0}`

#### Scenario: The web job has one check step

- **WHEN** a reviewer runs `yq '.jobs.web.steps[] | select(has("run")) | .run' .github/workflows/v2.yml`
- **THEN** the output is exactly one line: `nix develop --command just check-web`

#### Scenario: The web job has no step that installs a tool

- **WHEN** a reviewer runs `yq '.jobs.web.steps[] | select(has("uses")) | .uses | sub("@.*", "")' .github/workflows/v2.yml`
- **THEN** the output is exactly two lines, in this order: `actions/checkout` and `DeterminateSystems/determinate-nix-action`

#### Scenario: The local gate runs the same recipe

- **WHEN** a contributor runs `just --show check`
- **THEN** the output contains the line `check: check-server check-web`

#### Scenario: A check in the recipe fails

- **WHEN** a pull request changes a file under `server/crates/` so that `cargo fmt --check` reports a difference
- **THEN** the `server` job fails on that pull request

### Requirement: The v2 web checks run when v2 code changes

A CI job named `web`, in the workflow file `.github/workflows/v2.yml`, SHALL run the v2 web checks on every pull request that triggers that workflow. These are the same files that trigger the `server` job.

The checks are the type check, the lint, the format check, and the unit tests of the web shell in `web/`. The job SHALL fail when any check fails.

#### Scenario: A pull request changes web code

- **WHEN** a pull request changes a file under `web/src/`
- **THEN** the `web` job runs on that pull request

#### Scenario: A pull request changes only server code

- **WHEN** a pull request changes files under `server/` only
- **THEN** the `web` job runs on that pull request

#### Scenario: A pull request changes only v0.10.x code

- **WHEN** a pull request changes files under `apps/` only
- **THEN** GitHub does not run the `web` job on that pull request

#### Scenario: A web check fails

- **WHEN** a pull request changes a file under `web/src/` so that the format check reports a difference
- **THEN** the `web` job fails on that pull request
