## MODIFIED Requirements

### Requirement: The Nix shell provides the OpenSpec CLI

The Nix development shell SHALL provide the `openspec` CLI. `just skills` SHALL stop before changing any file when `openspec` is not on `PATH`, and its error message SHALL name the command that installs it. Every generated `SKILL.md` SHALL record the CLI version that generated it.

#### Scenario: The CLI is missing

- **WHEN** a contributor without `openspec` on `PATH` runs `just skills`
- **THEN** the command exits with a non-zero status
- **AND** its error message names `npm install -g @fission-ai/openspec`
- **AND** `git status --porcelain` shows the same output as before the run

#### Scenario: Generated skills record the CLI version

- **WHEN** a contributor runs `just skills`
- **THEN** every generated `SKILL.md` has `generatedBy` equal to the output of `openspec --version`

#### Scenario: The Nix shell provides the CLI

- **WHEN** a contributor runs `nix develop --command just skills`
- **THEN** the command exits with status 0

### Requirement: The mise lock pins every other tool's exact release

`mise.lock` SHALL be committed. It SHALL record one exact release for each tool `mise.toml` names, except Rust. For each tool mise downloads as a file, the lock SHALL record a download address and checksum for at least `linux-x64`, `linux-arm64` and `macos-arm64`. `mise install` SHALL install the releases the lock records, and SHALL NOT change the lock.

#### Scenario: The lock is up to date

- **WHEN** a reviewer runs `mise lock` on a clean checkout, then `git diff --exit-code mise.lock`
- **THEN** the `git diff` command exits with status 0

#### Scenario: Installing leaves the lock unchanged

- **WHEN** a contributor runs `mise install` on a clean checkout, then `git status --porcelain mise.lock`
- **THEN** the output is empty

#### Scenario: The lock covers the supported systems

- **WHEN** a reviewer reads the `[[tools.node]]`, `[[tools.pnpm]]`, `[[tools.just]]`, `[[tools.gh]]`, `[[tools."github:Fission-AI/OpenSpec"]]` and `[[tools."github:sidereal-io/lets-spec"]]` entries in `mise.lock`
- **THEN** each entry has a `platforms.linux-x64`, a `platforms.linux-arm64` and a `platforms.macos-arm64` table
- **AND** each of those tables has a `url` and a `checksum`

#### Scenario: The lock pins Let's Spec at one release

- **WHEN** a reviewer reads the `[[tools."github:sidereal-io/lets-spec"]]` entry in `mise.lock`
- **THEN** its `version` is `0.8.0`

#### Scenario: The lock pins OpenSpec at one release

- **WHEN** a reviewer reads the `[[tools."github:Fission-AI/OpenSpec"]]` entry in `mise.lock`
- **THEN** its `version` is `2.1.1`
- **AND** the `checksum` of each of its `linux-x64`, `linux-arm64` and `macos-arm64` tables equals the line for that file in the `SHA256SUMS` file of the OpenSpec 2.1.1 release

#### Scenario: mise needs no dependency lock for OpenSpec

- **WHEN** a reviewer runs `git ls-files .mise` on a clean checkout
- **THEN** the output is empty

### Requirement: Entering the repo with mise refreshes the skills

For a contributor who has run `mise activate` in their shell and trusted the repo's `mise.toml`, entering the repo directory SHALL run `lets sync`. When the committed copies already match the release, entering SHALL change no file. It SHALL print nothing from `lets sync` except the OpenSpec version warning that the requirement "Entering the repo warns while the skills lag OpenSpec 2" allows.

#### Scenario: A Let's Spec skill was deleted

- **WHEN** a contributor with mise activated deletes `.agents/skills/lets-plan/SKILL.md`, leaves the repo directory, and enters it again
- **THEN** `.agents/skills/lets-plan/SKILL.md` exists
- **AND** `git status --porcelain` prints nothing

#### Scenario: Nothing changed

- **WHEN** a contributor with mise activated enters the repo directory on a clean checkout
- **THEN** `git status --porcelain` prints nothing
- **AND** `lets sync` writes nothing to standard output
- **AND** `lets sync` writes nothing to standard error except the OpenSpec version warning

### Requirement: mise sets the Let's Spec flow and keeps the OpenSpec CLI offline

`mise.toml` SHALL set `LETS_SPEC_FLOW` to `standard` and `OPENSPEC_NO_UPDATE_CHECK` to `1`, so no `openspec` command run through mise checks for a newer version.

#### Scenario: A contributor reads the environment through mise

- **WHEN** a contributor runs `mise exec -- sh -c 'printenv LETS_SPEC_FLOW; printenv OPENSPEC_NO_UPDATE_CHECK'` at the repo root
- **THEN** the output is two lines: `standard`, then `1`

### Requirement: CI checks the mise route on every pull request that changes it

A CI job, the mise check, SHALL run on every pull request that changes one of these files:

- `mise.toml` or `mise.lock`;
- `server/rust-toolchain.toml` or `.nvmrc`;
- the workflow file that defines the mise check.

The mise check SHALL install the tools from `mise.lock`. It SHALL fail when an install fails.

#### Scenario: A pull request breaks the lock

- **WHEN** a pull request changes the `checksum` of the `linux-x64` entry for `just` in `mise.lock` to a wrong value
- **THEN** the mise check on that pull request fails

#### Scenario: A pull request does not touch the mise route

- **WHEN** a pull request changes none of the files in this requirement's list
- **THEN** GitHub does not run the mise check on that pull request

## ADDED Requirements

### Requirement: The Nix shell runs the pinned OpenSpec release

The Nix development shell's `openspec` SHALL be an OpenSpec release binary from GitHub. The Nix files SHALL pin that release, and the hash of its download for each supported system. That copy SHALL come ahead on `PATH` of any other `openspec` the shell's packages carry.

#### Scenario: The Nix shell runs the pinned release

- **WHEN** a contributor runs `nix develop --command openspec --version` at the repo root
- **THEN** the output is `2.1.1`

#### Scenario: A download does not match its hash

- **WHEN** a tester changes the hash the Nix files record for the `x86_64-linux` OpenSpec download to a wrong value, then runs `nix develop --command true` on an `x86_64-linux` machine
- **THEN** the command exits with a non-zero status
- **AND** its error output reports a hash mismatch

### Requirement: Entering the repo warns while the skills lag OpenSpec 2

Until a Let's Spec release names OpenSpec 2, the `openspec-*` skills that `lets sync` writes come from OpenSpec 1.13.2. Until then, on a clean checkout, `lets sync` SHALL exit with status 0 and write to standard error only its warning that the OpenSpec CLI on `PATH` differs from that version. Entering the repo SHALL show this warning on both routes.

#### Scenario: A contributor enters a clean checkout with mise

- **WHEN** a contributor with mise activated enters the repo directory on a clean checkout
- **THEN** `lets sync` writes to standard error exactly two lines: `lets-spec: 1 problem(s) to fix:`, then `- the OpenSpec CLI on PATH is 2.1.1, but this release's openspec-* skills were generated from 1.13.2. Pin the CLI to 1.13.2.`
- **AND** `git status --porcelain` prints nothing

#### Scenario: CI checks the committed copies

- **WHEN** a reviewer runs `mise exec -- lets sync --check` on a clean checkout
- **THEN** the command exits with status 0

### Requirement: The flake check fails when the shell runs another OpenSpec release

The flake check SHALL fail when `openspec --version` in the development shell prints a version other than the release the Nix files pin. The pinned release SHALL be named in one place in the Nix files.

#### Scenario: Another copy of the CLI comes first

- **WHEN** a tester changes the Nix files so that the Let's Spec bundle's `openspec` comes ahead of the pinned release on the shell's `PATH`, then runs `nix flake check`
- **THEN** the command exits with a non-zero status
- **AND** its error output names both versions

#### Scenario: The shell runs the pinned release

- **WHEN** a tester runs `nix flake check` on a clean checkout
- **THEN** the command exits with status 0
