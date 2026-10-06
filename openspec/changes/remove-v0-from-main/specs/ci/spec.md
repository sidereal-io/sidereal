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

A CI job named `server` SHALL run the server checks. The job lives in the workflow file `.github/workflows/ci.yml`. It SHALL run on every pull request, and on every push to `main`, that changes one of these files:

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

A CI job named `web` SHALL run the web checks. The job lives in the workflow file `.github/workflows/ci.yml`. It SHALL run on every pull request, and on every push to `main`, that triggers that workflow. The same files trigger the `server` job.

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

### Requirement: Main runs only its own workflows and the v0.x scan

The `.github/workflows/` folder on `main` SHALL hold exactly five workflow files: `ci.yml`, `codeql.yml`, `nix.yml`, `prune-ghcr.yml`, and `v0-security-scan.yml`. No workflow on `main` SHALL build or push a container image, or publish a release.

`prune-ghcr.yml` and `v0-security-scan.yml` stay on `main` because GitHub runs scheduled workflows from the default branch only.

#### Scenario: A reviewer lists the workflows

- **WHEN** a reviewer lists `.github/workflows/` on `main`
- **THEN** the folder holds exactly `ci.yml`, `codeql.yml`, `nix.yml`, `prune-ghcr.yml`, and `v0-security-scan.yml`

#### Scenario: A merge to main publishes no image

- **WHEN** a commit is pushed to `main`
- **THEN** no workflow run pushes an image to `ghcr.io/sidereal-io/sidereal`

#### Scenario: A version tag is pushed from main

- **WHEN** someone pushes a tag that matches `v*.*.*` and points at a commit on `main`
- **THEN** GitHub starts no release workflow for that tag

### Requirement: Each workflow grants only the token access it needs

`ci.yml` SHALL grant its jobs `contents: read` and no other permission. `codeql.yml` and `v0-security-scan.yml` SHALL grant `contents: read` and `security-events: write`, and no other permission. None of these three workflows SHALL refer to a repository secret.

The scenarios below use `yq` version 4 (the Go implementation). The development shell does not provide it, so a reviewer installs it first.

#### Scenario: The CI workflow can only read

- **WHEN** a reviewer runs `yq '[.permissions, .jobs[].permissions] | map(select(. != null))' .github/workflows/ci.yml`
- **THEN** the output lists only `contents: read`

#### Scenario: The scanning workflows can only read code and write results

- **WHEN** a reviewer runs the same command on `codeql.yml` and on `v0-security-scan.yml`
- **THEN** each output lists only `contents: read` and `security-events: write`

#### Scenario: No workflow uses a secret

- **WHEN** a reviewer runs `grep -n 'secrets' .github/workflows/ci.yml .github/workflows/codeql.yml .github/workflows/v0-security-scan.yml`
- **THEN** the search finds no match

### Requirement: A weekly scan checks v0.x for known vulnerabilities

Dependabot alerts read the default branch only, so they do not cover `v0.x`. The workflow `v0-security-scan.yml` fills that gap for the `v0.x` branch's dependency manifests, such as its npm lockfile. It does not scan container images. `v0.x` scans its image on every push and pull request.

The workflow SHALL meet these rules:

- It runs once a week, on Monday, and on manual dispatch.
- It checks out the `v0.x` branch into the folder `v0x`, and scans that folder's files only.
- It uploads its results to code scanning with the ref `refs/heads/v0.x`, the commit SHA it checked out, and the category `v0.x-dependencies`.
- After the upload, it fails the run when it finds a vulnerability rated high or critical. GitHub then reports the failed run to the maintainer.
- Every action it uses from outside GitHub's `actions/` and `github/` organisations is pinned to a full 40-character commit SHA.

The scenarios below use `yq` version 4 (the Go implementation). The development shell does not provide it, so a reviewer installs it first.

#### Scenario: The scan runs every Monday

- **WHEN** a reviewer runs `yq '.on | keys' .github/workflows/v0-security-scan.yml`
- **THEN** the output lists exactly `schedule` and `workflow_dispatch`
- **AND** `yq '.on.schedule[].cron' .github/workflows/v0-security-scan.yml` prints exactly one line, with five fields, whose fifth field is `1`

#### Scenario: The scan checks out v0.x into its own folder

- **WHEN** a reviewer runs `yq '.jobs[].steps[] | select(.uses == "actions/checkout*") | .with | [.ref, .path]' .github/workflows/v0-security-scan.yml`
- **THEN** the output lists exactly `v0.x` and `v0x`

#### Scenario: Every scan step reads the v0.x folder

- **WHEN** a reviewer runs `yq '.jobs[].steps[] | select(.uses == "aquasecurity/trivy-action*") | .with | [."scan-type", ."scan-ref"]' .github/workflows/v0-security-scan.yml`
- **THEN** every step in the output lists `fs` and `v0x`

#### Scenario: The upload names v0.x and the commit it scanned

- **WHEN** a reviewer runs `yq '.jobs[].steps[] | select(.uses == "github/codeql-action/upload-sarif*") | .with' .github/workflows/v0-security-scan.yml`
- **THEN** the output sets `ref` to `refs/heads/v0.x` and `category` to `v0.x-dependencies`
- **AND** it sets `sha` to the `commit` output of the checkout step

#### Scenario: A high or critical finding fails the run

- **WHEN** a reviewer runs `yq '.jobs[].steps[] | select(.with."exit-code" == "1") | .with.severity' .github/workflows/v0-security-scan.yml`
- **THEN** the output is exactly one line: `HIGH,CRITICAL`
- **AND** that step comes after the upload step in the job

#### Scenario: Third-party actions are pinned

- **WHEN** a reviewer lists every `uses:` value in `v0-security-scan.yml` that does not start with `actions/` or `github/`
- **THEN** every value ends with `@` followed by 40 hexadecimal characters

#### Scenario: A manual run uploads results for the current v0.x commit

- **WHEN** the maintainer runs `v0-security-scan.yml` by hand on `main`, and the run finishes
- **AND** runs `gh api 'repos/sidereal-io/sidereal/code-scanning/analyses?ref=refs/heads/v0.x' --jq '[.[] | select(.category == "v0.x-dependencies")][0] | .commit_sha, .created_at'`
- **THEN** the first printed line equals the output of `git rev-parse origin/v0.x` after `git fetch origin v0.x`
- **AND** the second printed line is a time later than the moment the run started
