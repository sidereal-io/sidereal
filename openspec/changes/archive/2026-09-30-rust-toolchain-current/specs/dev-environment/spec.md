## MODIFIED Requirements

### Requirement: The shell provides the pinned toolchain

The development shell SHALL provide these tools on `PATH`:

- Rust at a stable release, with `rustfmt` and `clippy`;
- Node at major version 26;
- `just`;
- the `openspec` CLI.

#### Scenario: A contributor checks tool versions in the shell

- **WHEN** a contributor runs `nix develop --command sh -c 'rustc --version; node --version; just --version; openspec --version'` at the repo root
- **THEN** `rustc --version` output starts with `rustc 1.`
- **AND** the `rustc --version` output contains neither `beta` nor `nightly`
- **AND** `node --version` output starts with `v26.`
- **AND** the `just` and `openspec` commands each exit with status 0

#### Scenario: Formatting and lint tools come with Rust

- **WHEN** a contributor runs `nix develop --command sh -c 'cargo fmt --version && cargo clippy --version'`
- **THEN** both commands exit with status 0

### Requirement: The shell turns on when you enter the repo

For a contributor with Nix and direnv, entering the repo directory SHALL turn the development shell on after a one-time `direnv allow`. Leaving the directory SHALL restore the contributor's previous environment. The shell SHALL reload when a file that defines it changes. The shell SHALL also reload when a file that the skills are generated from changes: `.config/openspec/config.json` or `scripts/skills.sh`.

#### Scenario: A contributor enters and leaves the repo

- **WHEN** a contributor with Nix and direnv has run `direnv allow` once, then changes into the repo directory
- **THEN** `rustc --version` reports the same release as `nix develop --command rustc --version`, without any further command
- **AND** after the contributor changes to a directory outside the repo, `rustc --version` reports the same result as before they entered it

#### Scenario: The Rust pin changes while the shell is active

- **WHEN** a tester with the shell loaded changes `channel` in `server/rust-toolchain.toml` from `stable` to `1.97.1`, without leaving the repo directory
- **AND** runs `direnv export bash` in the repo directory
- **THEN** `rustc --version` in that shell reports 1.97.1, not the release from before the edit

#### Scenario: The watch list covers the shell's files

- **WHEN** a tester with the shell loaded runs `direnv status` in the repo directory
- **THEN** the output lists `server/rust-toolchain.toml` and each file under `nix/` as watched
- **AND** the output lists `.config/openspec/config.json` and `scripts/skills.sh` as watched

### Requirement: The flake check reports the shell's tool versions

After the shell builds, the flake check SHALL run `rustc --version`, `openspec --version`, `node --version`, and `just --version` inside the shell. A reviewer SHALL be able to read their output in the job log. The flake check SHALL fail when any of the four commands fails.

#### Scenario: A reviewer checks what a pin update changed

- **WHEN** the flake check passes on a pull request
- **THEN** the job log contains the output of `rustc --version`, `openspec --version`, `node --version`, and `just --version`, each run inside the development shell

#### Scenario: A pin update breaks a tool

- **WHEN** `openspec --version` exits with a non-zero status inside the shell, and the other three commands succeed
- **THEN** the flake check fails

## ADDED Requirements

### Requirement: Rust follows the latest stable release

`server/rust-toolchain.toml` SHALL name the `stable` channel and no exact release. The development shell SHALL use the newest stable release that its locked inputs know. A new Rust release SHALL reach the development shell only through a change to `flake.lock`.

#### Scenario: The toolchain file names the stable channel

- **WHEN** a reviewer runs `grep -n '^channel' server/rust-toolchain.toml`
- **THEN** the output is exactly one line, `channel = "stable"`, after its line number

#### Scenario: A lock update brings a new Rust release

- **WHEN** a pull request changes only `flake.lock`, and the new lock knows a newer stable Rust release than the old lock
- **THEN** `nix develop --command rustc --version`, run on that pull request's commit, reports the newer release

#### Scenario: The same lock gives the same Rust release

- **WHEN** a tester runs `nix develop --command rustc --version` on one commit, on two days with different latest Rust releases
- **THEN** both runs report the same release

### Requirement: No file names a Rust release

The repo SHALL name no Rust release number in its build files or its contributor docs. The only Rust version setting SHALL be the channel in `server/rust-toolchain.toml`. The server image SHALL name only Rust major version 1 in its base image, and SHALL still build.

#### Scenario: The Cargo manifest sets no minimum Rust version

- **WHEN** a reviewer runs `grep -rn 'rust-version' server --include=Cargo.toml`
- **THEN** the search finds no match

#### Scenario: The server image names only the major version

- **WHEN** a reviewer runs `grep -n '^FROM rust:' server/Dockerfile`
- **THEN** every matching line names an image tag that starts with `rust:1-`

#### Scenario: The server image builds

- **WHEN** a reviewer runs `docker build server/` at the repo root
- **THEN** the command exits with status 0

#### Scenario: The contributor docs name no Rust release

- **WHEN** a reviewer runs `grep -nE '\b1\.[0-9]{2}(\.[0-9]+)?\b' AGENTS.md README.md server/README.md CONTRIBUTING.md`
- **THEN** the search finds no match

## REMOVED Requirements

### Requirement: The Rust version has one source of truth

**Reason**: `server/rust-toolchain.toml` no longer names an exact Rust release. "Rust follows the latest stable release" and "No file names a Rust release" replace this requirement. The lock file decides the exact release, as the existing requirement "The lock file decides every tool version" already requires for every tool.

**Migration**: To use a newer Rust release, merge the weekly Dependabot `flake.lock` pull request, or run `nix flake update rust-overlay`. To try a specific release locally, set `channel` to that release, and don't commit the edit.
