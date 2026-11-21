<!-- Plain language: Name the actor, condition and observable result. Explain necessary domain terms using the agreed glossary. Keep requirement and scenario headings and WHEN/THEN syntax exactly as required. -->

## MODIFIED Requirements

### Requirement: The shell provides the pinned toolchain

The development shell SHALL provide these tools on `PATH`:

- Rust at a stable release, with `rustfmt` and `clippy`;
- Node at major version 26;
- pnpm at major version 12;
- the `openspec` CLI.

#### Scenario: A contributor checks tool versions in the shell

- **WHEN** a contributor runs `nix develop --command sh -c 'rustc --version; node --version; pnpm --version; openspec --version'` at the repo root
- **THEN** `rustc --version` output starts with `rustc 1.`
- **AND** the `rustc --version` output contains neither `beta` nor `nightly`
- **AND** `node --version` output starts with `v26.`
- **AND** `pnpm --version` output starts with `12.`
- **AND** the `openspec` command exits with status 0

#### Scenario: Formatting and lint tools come with Rust

- **WHEN** a contributor runs `nix develop --command sh -c 'cargo fmt --version && cargo clippy --version'`
- **THEN** both commands exit with status 0

### Requirement: The repo checks pass through mise

The repo's gate SHALL pass when a contributor runs it through mise. The gate is `mise run check`: the Compose configuration checks, the OpenSpec checks, the server checks, and the web checks. The Nix development shell does not provide mise, so a contributor on the Nix route also runs the gate through mise.

#### Scenario: A contributor runs the gate in the shell

- **GIVEN** a prepared PostgreSQL fixture, with `TEST_DATABASE_URL` set to the URL `mise run db-test-url` prints
- **WHEN** a contributor runs `mise run check` at the repo root, on a commit whose `ci` workflow passes
- **THEN** the command exits with status 0

### Requirement: Every supported agent sees the same skills

Claude Code SHALL see the same skill set as agents that read `.agents/skills`. That set SHALL include both the skills Let's Spec writes and the skills people write in this repo. The repo SHALL keep one copy of each skill.

#### Scenario: Claude sees the generated and authored skills

- **WHEN** a reviewer reads a clean checkout
- **THEN** `.claude/skills/openspec-propose/SKILL.md` and `.claude/skills/visual-options/SKILL.md` both exist
- **AND** `.claude/skills` resolves to the same directory as `.agents/skills`

### Requirement: The flake check reports the shell's tool versions

After the shell builds, the flake check SHALL run `rustc --version`, `lets --version`, `openspec --version`, `node --version`, and `pnpm --version` inside the shell. A reviewer SHALL be able to read their output in the job log. The flake check SHALL fail when any of the five commands fails.

#### Scenario: A reviewer checks what a pin update changed

- **WHEN** the flake check passes on a pull request
- **THEN** the job log contains the output of `rustc --version`, `lets --version`, `openspec --version`, `node --version`, and `pnpm --version`, each run inside the development shell

#### Scenario: A pin update breaks a tool

- **WHEN** `openspec --version` exits with a non-zero status inside the shell, and the other four commands succeed
- **THEN** the flake check fails

### Requirement: mise provides the pinned toolchain

A contributor with mise SHALL get these tools, after running `mise trust` and `mise install` once at the repo root:

- Rust at a stable release, with `rustfmt` and `clippy`;
- Node at the major version `.nvmrc` names;
- pnpm at major version 12;
- `gh`, at 2.102.0 or later;
- the `openspec` CLI;
- the `lets` command from Let's Spec.

#### Scenario: A contributor checks tool versions through mise

- **WHEN** a contributor runs `mise exec -- sh -c 'rustc --version; node --version; pnpm --version; gh --version; openspec --version; lets --version'` at the repo root
- **THEN** `rustc --version` output starts with `rustc 1.` and contains neither `beta` nor `nightly`
- **AND** `node --version` output starts with `v26.`
- **AND** `pnpm --version` output starts with `12.`
- **AND** the `gh`, `openspec` and `lets` commands each exit with status 0

#### Scenario: Formatting and lint tools come with Rust

- **WHEN** a contributor runs `mise exec -- sh -c 'cargo fmt --version && cargo clippy --version'` at the repo root
- **THEN** both commands exit with status 0

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

- **WHEN** a reviewer reads the `[[tools."github:sidereal-io/lets-spec"]]` entry in `mise.lock`
- **THEN** its `version` is `0.8.0`

#### Scenario: The lock pins OpenSpec at one release

- **WHEN** a reviewer reads the `[[tools."github:Fission-AI/OpenSpec"]]` entry in `mise.lock`
- **THEN** its `version` is `2.1.1`
- **AND** the `checksum` of each of its `linux-x64`, `linux-arm64` and `macos-arm64` tables equals the line for that file in the `SHA256SUMS` file of the OpenSpec 2.1.1 release

#### Scenario: mise needs no dependency lock for OpenSpec

- **WHEN** a reviewer runs `git ls-files .mise` on a clean checkout
- **THEN** the output is empty

### Requirement: CI checks the mise route on every pull request that changes it

A CI job, the mise check, SHALL run on every pull request that changes one of these files:

- `mise.toml` or `mise.lock`;
- `server/rust-toolchain.toml` or `.nvmrc`;
- the workflow file that defines the mise check.

The mise check SHALL install the tools from `mise.lock`. It SHALL fail when an install fails.

#### Scenario: A pull request breaks the lock

- **WHEN** a pull request changes the `checksum` of the `linux-x64` entry for `pnpm` in `mise.lock` to a wrong value
- **THEN** the mise check on that pull request fails

#### Scenario: A pull request does not touch the mise route

- **WHEN** a pull request changes none of the files in this requirement's list
- **THEN** GitHub does not run the mise check on that pull request

### Requirement: The mise check reports the tool versions

After the install, the mise check SHALL run `rustc --version`, `node --version`, `pnpm --version`, `gh --version`, `openspec --version` and `lets --version` through mise. A reviewer SHALL be able to read their output in the job log. The mise check SHALL fail when any of the six commands fails.

#### Scenario: A reviewer checks what a pin update changed

- **WHEN** the mise check passes on a pull request
- **THEN** the job log contains the output of each of the six commands

#### Scenario: A pin update breaks a tool

- **WHEN** `openspec --version` exits with a non-zero status through mise, and the other five commands succeed
- **THEN** the mise check fails

### Requirement: The mise check fails when the lock misses a supported platform

The mise check SHALL fail when a `[[tools.*]]` entry in `mise.lock` lacks any of the `platforms.linux-x64`, `platforms.linux-arm64` or `platforms.macos-arm64` tables. The one exception SHALL be `[[tools.rust]]`, which records the `stable` channel and has no platform tables. The check's log SHALL name each tool that lacks a table and each platform it lacks. The check SHALL NOT require any other platform.

#### Scenario: A tool lacks a supported platform

- **WHEN** a pull request removes the `[tools.pnpm."platforms.macos-arm64"]` table from `mise.lock`
- **THEN** the mise check on that pull request fails
- **AND** its log names the tool `pnpm` and the platform `macos-arm64`

## REMOVED Requirements

### Requirement: The skills command gives the same skills on every machine

**Reason**: Commit 74da6ba removed the `just skills` and `just enter` recipes and `scripts/skills.sh`, when Let's Spec took over the agent skills. The `justfile` this requirement depends on is gone too.
**Migration**: Let's Spec writes and checks the skills. Run `lets sync`, or `mise exec -- lets sync --check` to check them; entering the repo with mise or the Nix shell runs `lets sync` for you. See the requirements "Entering the repo with mise refreshes the skills" and "Both routes use one Let's Spec release".

### Requirement: The skills command leaves the contributor's own config alone

**Reason**: Commit 74da6ba removed the `just skills` and `just enter` recipes and `scripts/skills.sh`, when Let's Spec took over the agent skills. The `justfile` this requirement depends on is gone too.
**Migration**: Let's Spec writes and checks the skills. Run `lets sync`, or `mise exec -- lets sync --check` to check them; entering the repo with mise or the Nix shell runs `lets sync` for you. See the requirements "Entering the repo with mise refreshes the skills" and "Both routes use one Let's Spec release".

### Requirement: The Nix shell provides the OpenSpec CLI

**Reason**: Commit 74da6ba removed the `just skills` and `just enter` recipes and `scripts/skills.sh`, when Let's Spec took over the agent skills. The `justfile` this requirement depends on is gone too.
**Migration**: Let's Spec writes and checks the skills. Run `lets sync`, or `mise exec -- lets sync --check` to check them; entering the repo with mise or the Nix shell runs `lets sync` for you. See the requirements "Entering the repo with mise refreshes the skills" and "Both routes use one Let's Spec release".

### Requirement: Git tracks authored skills and the skill settings

**Reason**: Commit 74da6ba removed the `just skills` and `just enter` recipes and `scripts/skills.sh`, when Let's Spec took over the agent skills. The `justfile` this requirement depends on is gone too.
**Migration**: Let's Spec writes and checks the skills. Run `lets sync`, or `mise exec -- lets sync --check` to check them; entering the repo with mise or the Nix shell runs `lets sync` for you. See the requirements "Entering the repo with mise refreshes the skills" and "Both routes use one Let's Spec release".

### Requirement: Skills work without Nix

**Reason**: Commit 74da6ba removed the `just skills` and `just enter` recipes and `scripts/skills.sh`, when Let's Spec took over the agent skills. The `justfile` this requirement depends on is gone too.
**Migration**: Let's Spec writes and checks the skills. Run `lets sync`, or `mise exec -- lets sync --check` to check them; entering the repo with mise or the Nix shell runs `lets sync` for you. See the requirements "Entering the repo with mise refreshes the skills" and "Both routes use one Let's Spec release".

### Requirement: Loading the shell refreshes stale skills

**Reason**: Commit 74da6ba removed the `just skills` and `just enter` recipes and `scripts/skills.sh`, when Let's Spec took over the agent skills. The `justfile` this requirement depends on is gone too.
**Migration**: Let's Spec writes and checks the skills. Run `lets sync`, or `mise exec -- lets sync --check` to check them; entering the repo with mise or the Nix shell runs `lets sync` for you. See the requirements "Entering the repo with mise refreshes the skills" and "Both routes use one Let's Spec release".

### Requirement: Loading the shell leaves current skills alone

**Reason**: Commit 74da6ba removed the `just skills` and `just enter` recipes and `scripts/skills.sh`, when Let's Spec took over the agent skills. The `justfile` this requirement depends on is gone too.
**Migration**: Let's Spec writes and checks the skills. Run `lets sync`, or `mise exec -- lets sync --check` to check them; entering the repo with mise or the Nix shell runs `lets sync` for you. See the requirements "Entering the repo with mise refreshes the skills" and "Both routes use one Let's Spec release".

### Requirement: Refreshing skills never blocks the shell

**Reason**: Commit 74da6ba removed the `just skills` and `just enter` recipes and `scripts/skills.sh`, when Let's Spec took over the agent skills. The `justfile` this requirement depends on is gone too.
**Migration**: Let's Spec writes and checks the skills. Run `lets sync`, or `mise exec -- lets sync --check` to check them; entering the repo with mise or the Nix shell runs `lets sync` for you. See the requirements "Entering the repo with mise refreshes the skills" and "Both routes use one Let's Spec release".

### Requirement: Contributors without Nix can opt in to the refresh

**Reason**: Commit 74da6ba removed the `just skills` and `just enter` recipes and `scripts/skills.sh`, when Let's Spec took over the agent skills. The `justfile` this requirement depends on is gone too.
**Migration**: Let's Spec writes and checks the skills. Run `lets sync`, or `mise exec -- lets sync --check` to check them; entering the repo with mise or the Nix shell runs `lets sync` for you. See the requirements "Entering the repo with mise refreshes the skills" and "Both routes use one Let's Spec release".
