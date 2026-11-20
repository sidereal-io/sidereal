<!-- Plain language: Name the actor, condition and observable result. Explain necessary domain terms using the agreed glossary. Keep requirement and scenario headings and WHEN/THEN syntax exactly as required. -->

## ADDED Requirements

### Requirement: A weekly workflow proposes mise tool updates

The workflow `.github/workflows/mise-tool-updates.yml` SHALL run every Monday at 04:00 UTC, and when someone starts it by hand from the Actions tab. It SHALL have no other trigger. Each run SHALL update the mise pins, refresh `mise.lock`, and run `lets sync`. When that changes any file, the run SHALL open a pull request against `main`. When the run changes no file, it SHALL open no pull request.

#### Scenario: A reviewer checks the triggers

- **WHEN** a reviewer runs `yq '.on | keys' .github/workflows/mise-tool-updates.yml`
- **THEN** the output lists exactly `schedule` and `workflow_dispatch`
- **AND** `yq '.on.schedule[].cron' .github/workflows/mise-tool-updates.yml` prints exactly one line: `0 4 * * 1`

#### Scenario: A tool has a newer release

- **WHEN** a run of the workflow on `main` finds a pnpm 12 release newer than the one `mise.lock` records
- **THEN** an open pull request against `main` changes the `[[tools.pnpm]]` entry in `mise.lock` to that release
- **AND** that pull request changes no file other than `mise.toml`, `mise.lock`, and files that `lets sync` writes

#### Scenario: Nothing has a newer release

- **WHEN** a run of the workflow on `main` finds no newer release for any tool it updates
- **THEN** the run finishes successfully
- **AND** the run opens no pull request

### Requirement: The weekly workflow keeps one update pull request

At most one pull request from the weekly workflow SHALL be open at a time. When one is already open and a run changes files, the run SHALL update that pull request instead of opening another.

#### Scenario: Last week's pull request is still open

- **WHEN** a pull request from the weekly workflow is open, and a later run finds a newer `gh` 2 release
- **THEN** that same pull request, with the same number, changes the `[[tools.gh]]` entry in `mise.lock` to the newer release
- **AND** exactly one pull request from the weekly workflow is open

### Requirement: The weekly update keeps each tool within its range

The weekly update SHALL move Node, pnpm and `gh` to their newest release within the major version that `.nvmrc` or `mise.toml` names. It SHALL move Let's Spec to its newest release, and change the version `mise.toml` names for it to match. It SHALL leave OpenSpec at the release `mise.toml` names, and Rust on the `stable` channel. It SHALL NOT change `.nvmrc`, `server/rust-toolchain.toml`, or the major version `mise.toml` names for pnpm or `gh`.

#### Scenario: Let's Spec has a newer release

- **WHEN** a run of the weekly workflow finds a Let's Spec release newer than the one `mise.lock` records
- **THEN** the pull request names that release for `github:sidereal-io/lets-spec` in both `mise.toml` and `mise.lock`
- **AND** `mise exec -- lets sync --check` exits with status 0 on the pull request's head commit

#### Scenario: A tool has a newer major version

- **WHEN** a run of the weekly workflow happens while pnpm 13 is the newest pnpm release
- **THEN** `mise.toml` on the pull request's head commit still names pnpm `12`
- **AND** the `[[tools.pnpm]]` entry in `mise.lock` names a release that starts with `12.`

#### Scenario: OpenSpec and Rust stay where they are

- **WHEN** a run of the weekly workflow opens or updates a pull request
- **THEN** the pull request does not change the `github:Fission-AI/OpenSpec` line in `mise.toml`, its entry in `mise.lock`, the `rust` line in `mise.toml`, `.nvmrc`, or `server/rust-toolchain.toml`

### Requirement: The weekly pull request runs CI

The weekly workflow SHALL open and update its pull request with a GitHub App token, so that GitHub runs the `ci` and `mise` workflows on it. The weekly workflow SHALL NOT run `mise run check` itself. When the updated tools fail the gate, the pull request SHALL still open, and the failure SHALL show in its checks.

#### Scenario: The weekly workflow opens a pull request

- **WHEN** the weekly workflow opens a pull request
- **THEN** GitHub runs the `ci` workflow's jobs and the mise check on that pull request
- **AND** the pull request's author is the dependency-merge App's bot account, not `github-actions[bot]`

#### Scenario: An update breaks the gate

- **WHEN** a run updates a tool to a release that makes `mise run check-web` fail
- **THEN** the run opens or updates the pull request
- **AND** the `web` job fails on that pull request

### Requirement: A failed update opens no pull request

When `mise lock` or `lets sync` fails during a run, the weekly workflow SHALL fail, and SHALL NOT open, update, or push to any pull request.

#### Scenario: lets sync fails

- **WHEN** `lets sync` exits with a non-zero status during a run of the weekly workflow
- **THEN** the run fails
- **AND** no pull request from the weekly workflow is opened or gains a commit from that run

### Requirement: Contributors can run the tool update by hand

`scripts/update-mise-tools.sh`, run at the repo root through mise, SHALL apply the same updates to the working tree as the weekly workflow. It SHALL NOT commit, push, or open a pull request. It SHALL exit with status 0 when every step succeeds, and with a non-zero status when `mise lock` or `lets sync` fails.

#### Scenario: A tester runs the update by hand

- **WHEN** a tester runs `mise exec -- scripts/update-mise-tools.sh` at the root of a clean checkout
- **THEN** the script exits with status 0
- **AND** `git status --porcelain` lists no file other than `mise.toml`, `mise.lock`, and files that `lets sync` writes
- **AND** `git rev-parse HEAD` prints the same commit as before the run

#### Scenario: The update leaves the lock complete

- **WHEN** a tester runs `mise exec -- scripts/update-mise-tools.sh` at the repo root, then `scripts/check-mise-lock-platforms.sh`
- **THEN** both commands exit with status 0

## MODIFIED Requirements

### Requirement: The mise lock pins every other tool's exact release

`mise.lock` SHALL be committed. It SHALL record one exact release for each tool `mise.toml` names, except Rust. For each tool mise downloads as a file, the lock SHALL record a download address and checksum for at least `linux-x64`, `linux-arm64` and `macos-arm64`. `mise install` SHALL install the releases the lock records, and SHALL NOT change the lock.

#### Scenario: The lock is up to date

- **WHEN** a reviewer runs `mise lock` on a clean checkout, then `git diff --exit-code mise.lock`
- **THEN** the `git diff` command exits with status 0

#### Scenario: Installing leaves the lock unchanged

- **WHEN** a contributor runs `mise install` on a clean checkout, then `git status --porcelain mise.lock`
- **THEN** the output is empty

#### Scenario: The lock covers the supported systems

- **WHEN** a reviewer reads the `[[tools.node]]`, `[[tools.pnpm]]`, `[[tools.gh]]`, `[[tools."github:Fission-AI/OpenSpec"]]` and `[[tools."github:sidereal-io/lets-spec"]]` entries in `mise.lock`
- **THEN** each entry has a `platforms.linux-x64`, a `platforms.linux-arm64` and a `platforms.macos-arm64` table
- **AND** each of those tables has a `url` and a `checksum`

#### Scenario: The lock pins Let's Spec at one release

- **WHEN** a reviewer reads the `[[tools."github:sidereal-io/lets-spec"]]` entry in `mise.lock`, and the `"github:sidereal-io/lets-spec"` line in `mise.toml`
- **THEN** the lock has exactly one such entry
- **AND** its `version` equals the version that `mise.toml` names

#### Scenario: The lock pins OpenSpec at one release

- **WHEN** a reviewer reads the `[[tools."github:Fission-AI/OpenSpec"]]` entry in `mise.lock`
- **THEN** its `version` is `2.1.1`
- **AND** the `checksum` of each of its `linux-x64`, `linux-arm64` and `macos-arm64` tables equals the line for that file in the `SHA256SUMS` file of the OpenSpec 2.1.1 release

#### Scenario: mise needs no dependency lock for OpenSpec

- **WHEN** a reviewer runs `git ls-files .mise` on a clean checkout
- **THEN** the output is empty
