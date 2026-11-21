<!-- Plain language: Name the actor, condition and observable result. Explain necessary domain terms using the agreed glossary. Keep requirement and scenario headings and WHEN/THEN syntax exactly as required. -->

## ADDED Requirements

### Requirement: The repo has no Nix route

The repo SHALL NOT define a Nix development shell or a direnv setup. It SHALL track no `flake.nix`, no `flake.lock`, no file under `nix/` and no `.envrc`. It SHALL have no workflow that runs `nix flake check`. `.github/dependabot.yml` SHALL have no entry for the `nix` package ecosystem.

#### Scenario: No Nix file is tracked

- **WHEN** a reviewer runs `git ls-files flake.nix flake.lock nix .envrc .github/workflows/nix.yml` at the repo root
- **THEN** the output is empty

#### Scenario: Dependabot has no Nix entry

- **WHEN** a reviewer runs `yq '.updates[] | select(."package-ecosystem" == "nix")' .github/dependabot.yml`
- **THEN** the output is empty

### Requirement: Leftover Nix-route files stay out of git

Git SHALL ignore `.direnv/` and `.envrc.local` at the repo root, so a contributor who used the retired Nix route cannot commit their direnv cache or personal settings by accident.

#### Scenario: A former Nix user stages everything

- **GIVEN** a checkout with a `.direnv/` directory and an `.envrc.local` file left from the Nix route
- **WHEN** the contributor runs `git add -A`, then `git status --porcelain`
- **THEN** the output lists neither `.direnv/` nor `.envrc.local`

### Requirement: Contributors can work without mise

The repo SHALL NOT require mise for building or checking it. The repo SHALL keep the pin files that tools other than mise read: `server/rust-toolchain.toml` for rustup, and `.nvmrc` for Node version managers.

#### Scenario: A contributor without mise runs the checks

- **GIVEN** a contributor without mise, who installs the tools by hand as `CONTRIBUTING.md` says, and prepares the PostgreSQL fixture
- **WHEN** the contributor installs Rust through rustup and installs Node through a version manager reading `.nvmrc`
- **THEN** rustup selects the channel in `server/rust-toolchain.toml`
- **AND** the version manager selects Node major version 26
- **AND** each of the four gate commands that `CONTRIBUTING.md` lists, for Compose, OpenSpec, the server and the web shell, exits with status 0 when run in that order from the repo root

### Requirement: mise loads no application settings

mise SHALL NOT load the variables in `.env` into a contributor's environment. `.env` holds the app's own settings, which can include secrets.

#### Scenario: A variable is set only in .env

- **WHEN** a tester adds `SIDEREAL_ENV_ONLY=1` to `.env`, then runs `mise exec -- printenv SIDEREAL_ENV_ONLY` at the repo root
- **THEN** the command prints nothing and exits with a non-zero status

### Requirement: The committed skills match the pinned Let's Spec release

The committed skills, schemas and rules block SHALL be the ones that `lets sync` writes for the Let's Spec release that `mise.toml` names.

#### Scenario: The committed copies match the release

- **WHEN** a reviewer runs `mise exec -- lets sync --check` on a clean checkout
- **THEN** the command exits with status 0

### Requirement: Rust follows the stable channel

`server/rust-toolchain.toml` SHALL name the `stable` channel and no exact release, so rustup installs the newest stable release it knows for a contributor without mise too.

#### Scenario: The toolchain file names the stable channel

- **WHEN** a reviewer runs `grep -n '^channel' server/rust-toolchain.toml`
- **THEN** the output is exactly one line, `channel = "stable"`, after its line number

#### Scenario: rustup without mise selects the stable channel

- **WHEN** a contributor with rustup and without mise runs `rustup show active-toolchain` in `server/`
- **THEN** the output starts with `stable-`

## MODIFIED Requirements

### Requirement: Contributors can add their own mise settings

mise SHALL apply the settings in a file named `mise.local.toml` at the repo root when it exists, over those in `mise.toml`. A folder that the file adds to `PATH` SHALL come ahead of every folder that mise manages, as a folder prepended in `.envrc.local` did on the retired Nix route. Git SHALL ignore `mise.local.toml`.

#### Scenario: A contributor adds a personal variable

- **WHEN** a contributor writes `[env]` and `SIDEREAL_TEST_VAR = "1"` in `mise.local.toml`, then runs `mise exec -- printenv SIDEREAL_TEST_VAR` at the repo root
- **THEN** the output is `1`

#### Scenario: A personal setting puts a folder first on PATH

- **GIVEN** a `.local-bin` directory at the repo root
- **WHEN** a contributor writes `[env]` and `_.path = ".local-bin"` in `mise.local.toml`, then runs `mise env --paths` at the repo root
- **THEN** the first line of the output is the absolute path of the repo's `.local-bin` directory

#### Scenario: The personal file stays out of git

- **WHEN** a contributor creates `mise.local.toml` and runs `git status --porcelain`
- **THEN** the output does not list `mise.local.toml`

### Requirement: The repo checks pass through mise

The repo's gate SHALL pass when a contributor runs it through mise. The gate is `mise run check`: the Compose configuration checks, the OpenSpec checks, the server checks, and the web checks.

#### Scenario: A contributor runs the gate in the shell

- **GIVEN** a prepared PostgreSQL fixture, with `TEST_DATABASE_URL` set to the URL `mise run db-test-url` prints
- **WHEN** a contributor runs `mise run check` at the repo root, on a commit whose `ci` workflow passes
- **THEN** the command exits with status 0

### Requirement: Contributors and CI name one Node version

The repo SHALL state Node major version 26 wherever it names a Node version for contributors. `.nvmrc` SHALL contain the major version only, so mise and Node version managers agree. A CI workflow that sets up Node without mise SHALL read `.nvmrc` rather than hold its own copy of the version, so CI can never drift from it. The app image SHALL build the web shell with a Node base image of the major version that `.nvmrc` names.

#### Scenario: A reviewer checks the documented Node version

- **WHEN** a reviewer reads `.nvmrc`
- **THEN** the file contains `26` and nothing else except a trailing newline

#### Scenario: No outdated Node version is left in the docs

- **WHEN** a reviewer searches `AGENTS.md`, `README.md`, `server/README.md`, `web/README.md` and `CONTRIBUTING.md` for a Node major version other than 26
- **THEN** the search finds no match

#### Scenario: A CI workflow that sets up Node reads .nvmrc

- **WHEN** a reviewer runs `grep -rn node-version .github/workflows/`
- **THEN** every matching line reads `node-version-file: '.nvmrc'`
- **AND** no matching line hardcodes a Node version number

#### Scenario: The app image builds the web shell with the same Node version

- **WHEN** a reviewer runs `grep -n '^FROM node:' Dockerfile` at the repo root
- **THEN** the search finds at least one line
- **AND** every matching line names an image tag that starts with `node:26-`

### Requirement: Entering the repo warns while the skills lag OpenSpec 2

Until a Let's Spec release names OpenSpec 2, the `openspec-*` skills that `lets sync` writes come from OpenSpec 1.13.2. Until then, on a clean checkout, `lets sync` SHALL exit with status 0 and write to standard error only its warning that the OpenSpec CLI on `PATH` differs from that version. Entering the repo with mise activated SHALL show this warning.

#### Scenario: A contributor enters a clean checkout with mise

- **WHEN** a contributor with mise activated enters the repo directory on a clean checkout
- **THEN** `lets sync` writes to standard error exactly two lines: `lets-spec: 1 problem(s) to fix:`, then `- the OpenSpec CLI on PATH is 2.1.1, but this release's openspec-* skills were generated from 1.13.2. Pin the CLI to 1.13.2.`
- **AND** `git status --porcelain` prints nothing

#### Scenario: CI checks the committed copies

- **WHEN** a reviewer runs `mise exec -- lets sync --check` on a clean checkout
- **THEN** the command exits with status 0

## REMOVED Requirements

### Requirement: Rust follows the latest stable release

**Reason**: It tied each new Rust release to a change in `flake.lock`, which is deleted.
**Migration**: "Rust follows the stable channel" keeps the `stable` channel in `server/rust-toolchain.toml`, and "Rust on the mise route follows rustup's stable channel" says how mise picks the release.

### Requirement: The shell provides the pinned toolchain

**Reason**: The Nix development shell is retired.
**Migration**: Install the tools with mise; "mise provides the pinned toolchain" lists the same tools. Without mise, install them by hand as `CONTRIBUTING.md` says.

### Requirement: The lock file decides every tool version

**Reason**: It held only for the Nix shell built from `flake.lock`, which is retired.
**Migration**: `mise.lock` records one exact release and a checksum for each tool except Rust; see "The mise lock pins every other tool's exact release".

### Requirement: The environment verifies remote code before running it

**Reason**: It covered flake inputs and scripts that `.envrc` loads. Both are retired.
**Migration**: `mise.lock` records a checksum for every download mise makes, and `mise install` refuses a download that does not match.

### Requirement: The shell turns on when you enter the repo

**Reason**: direnv no longer loads a Nix shell.
**Migration**: Run `mise activate` in your shell. "Entering the repo with mise refreshes the skills" covers what happens on entry.

### Requirement: Contributors can add their own direnv settings

**Reason**: The repo no longer has an `.envrc` that loads `.envrc.local`.
**Migration**: Move personal settings to `mise.local.toml`; see "Contributors can add their own mise settings". Git still ignores `.envrc.local`.

### Requirement: Nix stays optional

**Reason**: Nix is no longer a route, so it cannot be optional. The parts that still hold, the plain pin files and the hand-install route, move to "Contributors can work without mise". The rule that the environment does not load `.env` moves to "mise loads no application settings".
**Migration**: None needed.

### Requirement: The flake check builds the shell

**Reason**: The flake and its check are retired.
**Migration**: The mise check installs every tool from `mise.lock` on each pull request that changes the mise route.

### Requirement: The shell is defined for the supported systems

**Reason**: The flake is retired.
**Migration**: The supported systems are `linux-x64`, `linux-arm64` and `macos-arm64`; see "The mise lock pins every other tool's exact release".

### Requirement: The shell keeps the OpenSpec CLI offline

**Reason**: The Nix shell is retired.
**Migration**: mise sets `OPENSPEC_NO_UPDATE_CHECK=1`; see "mise sets the Let's Spec flow and keeps the OpenSpec CLI offline". Refreshing the skills without a network is no longer promised.

### Requirement: Dependabot proposes Nix pin updates every week

**Reason**: `flake.lock` is deleted, and Dependabot's `nix` entry goes with it.
**Migration**: The weekly mise tool update workflow proposes tool updates; see "A weekly workflow proposes mise tool updates".

### Requirement: CI builds the shell on every pull request that changes it

**Reason**: The `nix` workflow is deleted.
**Migration**: See "CI checks the mise route on every pull request that changes it".

### Requirement: The flake check runs without secrets

**Reason**: The `nix` workflow is deleted.
**Migration**: See "The mise check runs without secrets".

### Requirement: The flake check reports the shell's tool versions

**Reason**: The `nix` workflow is deleted.
**Migration**: See "The mise check reports the tool versions".

### Requirement: Both routes use one Let's Spec release

**Reason**: Only the mise route pins Let's Spec now.
**Migration**: "The mise lock pins every other tool's exact release" pins the release, and "The committed skills match the pinned Let's Spec release" keeps the committed copies in step with it.

### Requirement: The Nix shell runs the pinned OpenSpec release

**Reason**: The Nix shell is retired.
**Migration**: mise pins OpenSpec 2.1.1 with a checksum for each supported system; see "The mise lock pins every other tool's exact release".

### Requirement: The flake check fails when the shell runs another OpenSpec release

**Reason**: The flake check is retired.
**Migration**: The mise check runs `openspec --version` through mise and fails when it fails.
