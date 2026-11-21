## MODIFIED Requirements

### Requirement: No file names a Rust release

The repo SHALL name no Rust release number in its build files or its contributor docs. The only Rust version settings SHALL be the `stable` channel in `server/rust-toolchain.toml` and the `stable` channel in `mise.toml`. `mise.lock` SHALL record Rust as the `stable` channel, not as a release. The app image SHALL name only Rust major version 1 in its base image, and SHALL still build.

#### Scenario: The Cargo manifest sets no minimum Rust version

- **WHEN** a reviewer runs `grep -rn 'rust-version' server --include=Cargo.toml`
- **THEN** the search finds no match

#### Scenario: The mise configuration names the stable channel

- **WHEN** a reviewer runs `mise config get tools.rust.version` at the repo root
- **THEN** the output is `stable`

#### Scenario: The mise lock records the channel, not a release

- **WHEN** a reviewer reads the `[[tools.rust]]` entry in `mise.lock`
- **THEN** its `version` is `stable`
- **AND** its `specifiers` list is `["stable"]`

#### Scenario: The server image names only the major version

- **WHEN** a reviewer runs `grep -n '^FROM rust:' Dockerfile` at the repo root
- **THEN** the search finds at least one line
- **AND** every matching line names an image tag that starts with `rust:1-`

#### Scenario: The server image builds

- **WHEN** a reviewer runs `docker build .` at the repo root
- **THEN** the command exits with status 0

#### Scenario: The contributor docs name no Rust release

- **WHEN** a reviewer runs `grep -nE '\b1\.[0-9]{2}(\.[0-9]+)?\b' AGENTS.md README.md server/README.md CONTRIBUTING.md`
- **THEN** the search finds no match

## ADDED Requirements

### Requirement: mise provides the pinned toolchain

A contributor with mise SHALL get these tools, after running `mise trust` and `mise install` once at the repo root:

- Rust at a stable release, with `rustfmt` and `clippy`;
- Node at the major version `.nvmrc` names;
- pnpm at major version 12;
- `just`;
- `gh`, at 2.102.0 or later;
- the `openspec` CLI;
- the `lets` command from Let's Spec.

#### Scenario: A contributor checks tool versions through mise

- **WHEN** a contributor runs `mise exec -- sh -c 'rustc --version; node --version; pnpm --version; just --version; gh --version; openspec --version; lets --version'` at the repo root
- **THEN** `rustc --version` output starts with `rustc 1.` and contains neither `beta` nor `nightly`
- **AND** `node --version` output starts with `v26.`
- **AND** `pnpm --version` output starts with `12.`
- **AND** the `just`, `gh`, `openspec` and `lets` commands each exit with status 0

#### Scenario: Formatting and lint tools come with Rust

- **WHEN** a contributor runs `mise exec -- sh -c 'cargo fmt --version && cargo clippy --version'` at the repo root
- **THEN** both commands exit with status 0

### Requirement: mise takes the Node version from .nvmrc

`mise.toml` SHALL NOT name a Node version. mise SHALL read the Node version from `.nvmrc`, whatever the contributor's own mise settings say about version files.

#### Scenario: A contributor has no mise settings of their own

- **WHEN** a contributor whose global mise configuration is empty runs `mise ls --current node` at the repo root
- **THEN** the output names `.nvmrc` as the source of the Node version

#### Scenario: .nvmrc changes

- **WHEN** a tester changes `.nvmrc` to `24`, then runs `mise install` and `mise exec -- node --version` at the repo root
- **THEN** the output starts with `v24.`

### Requirement: Rust on the mise route follows rustup's stable channel

On the mise route, the Rust release SHALL be the newest stable release rustup knows when the contributor installs or updates it. The mise route SHALL NOT pin an exact Rust release.

#### Scenario: mise uses rustup's stable release

- **WHEN** a contributor runs `mise exec -- rustc --version` and `mise exec -- rustup run stable rustc --version` at the repo root
- **THEN** both commands print the same release

### Requirement: The mise lock pins every other tool's exact release

`mise.lock` SHALL be committed. It SHALL record one exact release for each tool `mise.toml` names, except Rust. For each tool mise downloads as a file, the lock SHALL record a download address and checksum for at least `linux-x64`, `linux-arm64` and `macos-arm64`. `mise install` SHALL install the releases the lock records, and SHALL NOT change the lock.

#### Scenario: The lock is up to date

- **WHEN** a reviewer runs `mise lock` on a clean checkout, then `git diff --exit-code mise.lock`
- **THEN** the `git diff` command exits with status 0

#### Scenario: Installing leaves the lock unchanged

- **WHEN** a contributor runs `mise install` on a clean checkout, then `git status --porcelain mise.lock`
- **THEN** the output is empty

#### Scenario: The lock covers the supported systems

- **WHEN** a reviewer reads the `[[tools.node]]`, `[[tools.pnpm]]`, `[[tools.just]]`, `[[tools.gh]]` and `[[tools."github:sidereal-io/lets-spec"]]` entries in `mise.lock`
- **THEN** each entry has a `platforms.linux-x64`, a `platforms.linux-arm64` and a `platforms.macos-arm64` table
- **AND** each of those tables has a `url` and a `checksum`

#### Scenario: The lock pins Let's Spec at one release

- **WHEN** a reviewer reads the `[[tools."github:sidereal-io/lets-spec"]]` entry in `mise.lock`
- **THEN** its `version` is `0.8.0`

### Requirement: Both routes use one Let's Spec release

The Nix shell and mise SHALL provide the same Let's Spec release and the same `openspec` CLI version. The flake input for Let's Spec SHALL name a release tag, not a branch. The committed skills, schemas and rules block SHALL be the ones that release's `lets sync` writes.

#### Scenario: The flake input names a tag

- **WHEN** a reviewer runs `nix flake metadata --json` at the repo root
- **THEN** the `lets-spec` input's `original` entry has `ref` set to `v0.8.0`

#### Scenario: The two routes report the same versions

- **WHEN** a tester runs `lets --version` and `openspec --version`, once through `nix develop --command` and once through `mise exec --`, at the repo root
- **THEN** the two `lets --version` outputs are identical
- **AND** the two `openspec --version` outputs are identical

#### Scenario: The committed copies match the release

- **WHEN** a reviewer runs `mise exec -- lets sync --check` on a clean checkout
- **THEN** the command exits with status 0

### Requirement: Entering the repo with mise refreshes the skills

For a contributor who has run `mise activate` in their shell and trusted the repo's `mise.toml`, entering the repo directory SHALL run `lets sync`. When the committed copies already match the release, entering SHALL print nothing from `lets sync` and change no file.

#### Scenario: A Let's Spec skill was deleted

- **WHEN** a contributor with mise activated deletes `.agents/skills/lets-plan/SKILL.md`, leaves the repo directory, and enters it again
- **THEN** `.agents/skills/lets-plan/SKILL.md` exists
- **AND** `git status --porcelain` prints nothing

#### Scenario: Nothing changed

- **WHEN** a contributor with mise activated enters the repo directory on a clean checkout
- **THEN** `git status --porcelain` prints nothing
- **AND** `lets sync` writes nothing to standard output or standard error

### Requirement: mise sets the Let's Spec flow and keeps the OpenSpec CLI offline

`mise.toml` SHALL set `LETS_SPEC_FLOW` to `standard` and `OPENSPEC_NO_UPDATE_CHECK` to `1`, so no `openspec` command run through mise checks the npm registry for a newer version.

#### Scenario: A contributor reads the environment through mise

- **WHEN** a contributor runs `mise exec -- sh -c 'printenv LETS_SPEC_FLOW; printenv OPENSPEC_NO_UPDATE_CHECK'` at the repo root
- **THEN** the output is two lines: `standard`, then `1`

### Requirement: Contributors can add their own mise settings

mise SHALL apply the settings in a file named `mise.local.toml` at the repo root when it exists, over those in `mise.toml`. Git SHALL ignore `mise.local.toml`.

#### Scenario: A contributor adds a personal variable

- **WHEN** a contributor writes `[env]` and `SIDEREAL_TEST_VAR = "1"` in `mise.local.toml`, then runs `mise exec -- printenv SIDEREAL_TEST_VAR` at the repo root
- **THEN** the output is `1`

#### Scenario: The personal file stays out of git

- **WHEN** a contributor creates `mise.local.toml` and runs `git status --porcelain`
- **THEN** the output does not list `mise.local.toml`

### Requirement: CI checks the mise route on every pull request that changes it

A CI job, the mise check, SHALL run on every pull request that changes one of these files:

- `mise.toml` or `mise.lock`;
- any file under `.mise/`, which holds the npm dependency lock;
- `server/rust-toolchain.toml` or `.nvmrc`;
- the workflow file that defines the mise check.

The mise check SHALL install the tools from `mise.lock`. It SHALL fail when an install fails.

#### Scenario: A pull request breaks the lock

- **WHEN** a pull request changes the `checksum` of the `linux-x64` entry for `just` in `mise.lock` to a wrong value
- **THEN** the mise check on that pull request fails

#### Scenario: A pull request does not touch the mise route

- **WHEN** a pull request changes none of the files in this requirement's list
- **THEN** GitHub does not run the mise check on that pull request

### Requirement: The mise check fails when the Rust channels differ

The mise check SHALL fail when the Rust channel in `mise.toml` differs from the `channel` in `server/rust-toolchain.toml`. Its log SHALL name both files.

#### Scenario: A pull request changes one channel only

- **WHEN** a pull request changes `channel` in `server/rust-toolchain.toml` to `beta`, and leaves `mise.toml` unchanged
- **THEN** the mise check on that pull request fails
- **AND** its log names `mise.toml` and `server/rust-toolchain.toml`

#### Scenario: The channels agree

- **WHEN** a pull request leaves both files naming `stable`, and the install succeeds
- **THEN** the mise check on that pull request passes

### Requirement: The mise check reports the tool versions

After the install, the mise check SHALL run `rustc --version`, `node --version`, `pnpm --version`, `just --version`, `gh --version`, `openspec --version` and `lets --version` through mise. A reviewer SHALL be able to read their output in the job log. The mise check SHALL fail when any of the seven commands fails.

#### Scenario: A reviewer checks what a pin update changed

- **WHEN** the mise check passes on a pull request
- **THEN** the job log contains the output of each of the seven commands

#### Scenario: A pin update breaks a tool

- **WHEN** `openspec --version` exits with a non-zero status through mise, and the other six commands succeed
- **THEN** the mise check fails

### Requirement: The mise check runs without secrets

The mise check SHALL use no repository secret. Its workflow SHALL grant the job read-only access to repository contents and no other permission.

#### Scenario: A reviewer inspects the workflow permissions

- **WHEN** a reviewer reads the mise check workflow file
- **THEN** its `permissions` block grants only `contents: read`
- **AND** the file contains no reference to `secrets.`
