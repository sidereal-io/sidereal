## Purpose

CI runs the checks that match what a pull request changes. v2 changes run the v2 checks, v0.10.x changes run the v0.10.x pipeline, and code scanning runs on every pull request.

## ADDED Requirements

### Requirement: The v2 server checks run when v2 server code changes

A CI job named `server`, in the workflow file `.github/workflows/v2.yml`, SHALL run the v2 server checks on every pull request that changes one of these files:

- any file under `server/`;
- the root `justfile`;
- `.github/workflows/v2.yml`.

The checks are the Rust format check, clippy with warnings denied, the tests, a build, and the dependency-direction lint. The job SHALL fail when any check fails.

#### Scenario: A pull request changes Rust code

- **WHEN** a pull request changes a file under `server/crates/`
- **THEN** the `server` job runs on that pull request
- **AND** the job fails if `cargo clippy --all-targets -- -D warnings` reports a warning

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

CodeQL SHALL analyze the repo on every pull request to `main` or `v0.x`, and on every push to those branches, whatever files the change touches. The CodeQL workflow SHALL have no path filter, so the `main` ruleset always finds the CodeQL results it requires.

#### Scenario: A v2-only pull request gets code scanning results

- **WHEN** a pull request to `main` changes files under `server/` only
- **THEN** the CodeQL workflow runs on that pull request and uploads its results
- **AND** the ruleset's code scanning rule does not block the merge for missing results

#### Scenario: The CodeQL workflow has no path filter

- **WHEN** a reviewer reads `.github/workflows/codeql.yml`
- **THEN** its `on` block contains no `paths` or `paths-ignore` key
- **AND** no other workflow file defines a CodeQL job
