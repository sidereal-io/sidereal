## RENAMED Requirements

- FROM: `### Requirement: Every v2 job runs one recipe inside the development shell`
- TO: `### Requirement: Every CI job runs one recipe inside the development shell`

## REMOVED Requirements

### Requirement: The v2 server checks run when v2 server code changes

**Reason**: Replaced by "The server checks run when server code changes". The workflow file is now `ci.yml`, it also runs on pushes to `main`, and no scenario mentions v0.10.x code.

**Migration**: None. The `server` job keeps its name, its trigger files, and its checks.

### Requirement: The v2 web checks run when v2 code changes

**Reason**: Replaced by "The web checks run when server or web code changes". The workflow file is now `ci.yml`, it also runs on pushes to `main`, and no scenario mentions v0.10.x code.

**Migration**: None. The `web` job keeps its name, its trigger files, and its checks.

### Requirement: The v0.10.x pipeline skips v2-only changes

**Reason**: The v0.10.x workflows `ci.yml` and `docker-build-push.yml` leave `main`. `main` has no v0.10.x pipeline left to skip.

**Migration**: None on `main`. The `v0.x` branch keeps its own copies of these workflows, and GitHub runs them for pushes and pull requests to `v0.x`.

### Requirement: Code scanning runs on every pull request

**Reason**: Replaced by "Code scanning covers the Rust and TypeScript code on main". CodeQL on `main` now scans Rust as well, and no longer names the `v0.x` branch.

**Migration**: None. The `v0.x` branch runs CodeQL from a job in its own `ci.yml`.

## MODIFIED Requirements

### Requirement: Every CI job runs one recipe inside the development shell

Every job in `.github/workflows/ci.yml` SHALL meet these four rules:

- The job runs its checks as one `just` recipe inside the Nix development shell.
- The job runs no check that the recipe does not run.
- The job has no step that installs Rust, Node, pnpm, or any other tool the checks use. The shell provides them.
- A step that caches cargo's build output runs its own Rust commands inside the shell, so it installs no Rust either.

The `server` job's recipe is `check-server`. The `web` job's recipe is `check-web`. `just check` SHALL run `check-server`, then `check-web`, so a contributor and CI run the same gates.

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

#### Scenario: The local gate runs the same recipe

- **WHEN** a contributor runs `just --show check`
- **THEN** the output contains the line `check: check-server check-web`

#### Scenario: A check in the recipe fails

- **WHEN** a pull request changes a file under `server/crates/` so that `cargo fmt --check` reports a difference
- **THEN** the `server` job fails on that pull request

## ADDED Requirements

### Requirement: The server checks run when server code changes

A CI job named `server`, in the workflow file `.github/workflows/ci.yml`, SHALL run the server checks on every pull request and every push to `main` that changes one of these files:

- any file under `server/`;
- any file under `web/`;
- the root `justfile`;
- `flake.nix` or `flake.lock`;
- any file under `nix/`;
- `.github/workflows/ci.yml`.

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

#### Scenario: A pull request changes only documentation

- **WHEN** a pull request changes files under `docs/` only
- **THEN** GitHub does not run the `server` job on that pull request

#### Scenario: A merge to main changes Rust code

- **WHEN** a commit that changes a file under `server/crates/` is pushed to `main`
- **THEN** the `server` job runs on that push

#### Scenario: The old workflow names are gone

- **WHEN** a reviewer lists `.github/workflows/`
- **THEN** `ci.yml` exists
- **AND** neither `v2.yml` nor `backend-rs.yml` exists

### Requirement: The web checks run when server or web code changes

A CI job named `web`, in the workflow file `.github/workflows/ci.yml`, SHALL run the web checks on every pull request and every push to `main` that triggers that workflow. These are the same files that trigger the `server` job.

The checks are the type check, the lint, the format check, and the unit tests of the web shell in `web/`. The job SHALL fail when any check fails.

#### Scenario: A pull request changes web code

- **WHEN** a pull request changes a file under `web/src/`
- **THEN** the `web` job runs on that pull request

#### Scenario: A pull request changes only server code

- **WHEN** a pull request changes files under `server/` only
- **THEN** the `web` job runs on that pull request

#### Scenario: A pull request changes only documentation

- **WHEN** a pull request changes files under `docs/` only
- **THEN** GitHub does not run the `web` job on that pull request

#### Scenario: A merge to main changes web code

- **WHEN** a commit that changes a file under `web/src/` is pushed to `main`
- **THEN** the `web` job runs on that push

#### Scenario: A web check fails

- **WHEN** a pull request changes a file under `web/src/` so that the format check reports a difference
- **THEN** the `web` job fails on that pull request

### Requirement: Code scanning covers the Rust and TypeScript code on main

CodeQL SHALL analyze the Rust code and the TypeScript code on every pull request to `main` and on every push to `main`, whatever files the change touches. The CodeQL workflow SHALL have no path filter. Every pull request to `main` therefore has CodeQL results for both languages.

The CodeQL workflow on `main` SHALL NOT name the `v0.x` branch. The `v0.x` branch runs its own code scanning.

#### Scenario: A server-only pull request gets results for both languages

- **WHEN** a pull request to `main` changes files under `server/` only
- **THEN** the CodeQL workflow runs on that pull request
- **AND** the pull request's checks list a completed CodeQL analysis for Rust and one for JavaScript and TypeScript

#### Scenario: The CodeQL workflow has no path filter

- **WHEN** a reviewer reads `.github/workflows/codeql.yml`
- **THEN** its `on` block contains no `paths` or `paths-ignore` key
- **AND** no other workflow file defines a CodeQL job

#### Scenario: The CodeQL workflow targets main only

- **WHEN** a reviewer runs `grep -n 'v0.x' .github/workflows/codeql.yml`
- **THEN** the search finds no match

### Requirement: Main runs no v0.10.x workflows

The `.github/workflows/` folder on `main` SHALL hold exactly four workflow files: `ci.yml`, `codeql.yml`, `nix.yml`, and `prune-ghcr.yml`. No workflow on `main` SHALL build or push a container image, or publish a release.

`prune-ghcr.yml` stays on `main` because GitHub runs scheduled workflows from the default branch only. It removes old images for both lines.

#### Scenario: A reviewer lists the workflows

- **WHEN** a reviewer lists `.github/workflows/` on `main`
- **THEN** the folder holds exactly `ci.yml`, `codeql.yml`, `nix.yml`, and `prune-ghcr.yml`

#### Scenario: A merge to main publishes no image

- **WHEN** a commit is pushed to `main`
- **THEN** no workflow run pushes an image to `ghcr.io/sidereal-io/sidereal`

#### Scenario: A version tag is pushed from main

- **WHEN** someone pushes a tag that matches `v*.*.*` and points at a commit on `main`
- **THEN** GitHub starts no release workflow for that tag
