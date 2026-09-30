## MODIFIED Requirements
### Requirement: The shell provides the pinned toolchain

The development shell SHALL provide these tools on `PATH`:

- Rust at the exact channel named in `server/rust-toolchain.toml`, with `rustfmt` and `clippy`;
- Node at major version 26;
- `just`;
- the `openspec` CLI.

#### Scenario: A contributor checks tool versions in the shell

- **WHEN** a contributor runs `nix develop --command sh -c 'rustc --version; node --version; just --version; openspec --version'` at the repo root
- **THEN** `rustc --version` reports the version in the `channel` field of `server/rust-toolchain.toml`
- **AND** `node --version` output starts with `v26.`
- **AND** the `just` and `openspec` commands each exit with status 0

#### Scenario: Formatting and lint tools come with Rust

- **WHEN** a contributor runs `nix develop --command sh -c 'cargo fmt --version && cargo clippy --version'`
- **THEN** both commands exit with status 0

### Requirement: The Rust version has one source of truth

The development shell SHALL take the Rust version and components from `server/rust-toolchain.toml` only. Changing the Rust version SHALL NOT require editing any other file.

#### Scenario: A maintainer bumps the Rust version

- **WHEN** a maintainer changes `channel` in `server/rust-toolchain.toml` to another stable version, and changes no other file
- **THEN** `nix develop --command rustc --version` reports the new version

### Requirement: The shell turns on when you enter the repo

For a contributor with Nix and direnv, entering the repo directory SHALL turn the development shell on after a one-time `direnv allow`. Leaving the directory SHALL restore the contributor's previous environment. The shell SHALL reload when a file that defines it changes. The shell SHALL also reload when a file that the skills are generated from changes: `.config/openspec/config.json` or `scripts/skills.sh`.

#### Scenario: A contributor enters and leaves the repo

- **WHEN** a contributor with Nix and direnv has run `direnv allow` once, then changes into the repo directory
- **THEN** `rustc --version` reports the pinned version without any further command
- **AND** after the contributor changes to a directory outside the repo, `rustc --version` reports the same result as before they entered it

#### Scenario: The Rust pin changes while the shell is active

- **WHEN** a tester with the shell loaded changes `channel` in `server/rust-toolchain.toml` to a different stable version, without leaving the repo directory
- **AND** runs `direnv export bash` in the repo directory
- **THEN** `rustc --version` in that shell reports the new version, not the one from before the edit

#### Scenario: The watch list covers the shell's files

- **WHEN** a tester with the shell loaded runs `direnv status` in the repo directory
- **THEN** the output lists `server/rust-toolchain.toml` and each file under `nix/` as watched
- **AND** the output lists `.config/openspec/config.json` and `scripts/skills.sh` as watched

### Requirement: Nix stays optional

The environment SHALL NOT require Nix for building or checking the repo. For a contributor with direnv but without Nix, entering the repo SHALL produce no error. It SHALL change no environment variables except direnv's own `DIRENV_*` variables and any that the contributor's `.envrc.local` sets. The repo SHALL keep the pin files that tools outside Nix read: `server/rust-toolchain.toml` for rustup, and `.nvmrc` for Node version managers.

#### Scenario: A contributor has direnv but not Nix

- **WHEN** a contributor without Nix and without `.envrc.local` runs `direnv allow`, then enters the repo directory
- **THEN** direnv reports no error
- **AND** `env`, with lines starting `DIRENV_` removed, shows the same output as before the contributor entered the directory

#### Scenario: A contributor without Nix runs the checks

- **WHEN** a contributor without Nix installs Rust through rustup and installs Node through a version manager reading `.nvmrc`
- **THEN** rustup selects the channel in `server/rust-toolchain.toml`
- **AND** the version manager selects Node major version 26
- **AND** `just check` exits with status 0, given that `just` is installed

#### Scenario: The environment does not load application settings

- **WHEN** a contributor enters the repo directory with the development shell turned on
- **THEN** no variable defined only in `.env` is set in the contributor's shell

### Requirement: Every reference to the Node version agrees

The repo SHALL state Node major version 26 wherever it names a Node version for contributors or for the production image. `.nvmrc` SHALL contain the major version only, so Nix and Node version managers agree. Every CI workflow SHALL read `.nvmrc` rather than hold its own copy of the version, so CI can never drift from it.

#### Scenario: A reviewer checks the documented Node version

- **WHEN** a reviewer reads `.nvmrc`
- **THEN** the file contains `26` and nothing else except a trailing newline

#### Scenario: No outdated Node version is left in the docs

- **WHEN** a reviewer searches `AGENTS.md`, `README.md`, `server/README.md` and `CONTRIBUTING.md` for a Node major version other than 26
- **THEN** the search finds no match

#### Scenario: Every CI workflow reads the Node version from .nvmrc

- **WHEN** a reviewer runs `grep -rn node-version .github/workflows/`
- **THEN** every matching line reads `node-version-file: '.nvmrc'`
- **AND** no matching line hardcodes a Node version number

#### Scenario: The production image names the same Node version as the shell

- **WHEN** a reviewer runs `grep -n 'FROM node:' Dockerfile`
- **THEN** every matching line names major version 26

### Requirement: Dependabot proposes Cargo updates every week

Dependabot SHALL check the crates in `server/Cargo.lock` every week. When a crate has a newer release, Dependabot SHALL open one grouped pull request for all the updated crates.

#### Scenario: The configuration declares the Cargo updates

- **WHEN** a reviewer runs `yq '.updates[] | select(."package-ecosystem" == "cargo")' .github/dependabot.yml`
- **THEN** the output is exactly one entry
- **AND** that entry has `directory` set to `/server` and `schedule.interval` set to `weekly`
- **AND** that entry has a group whose `patterns` list is `["*"]`

#### Scenario: Several crates have newer releases

- **WHEN** Dependabot's weekly run finds newer releases for two or more crates
- **THEN** one open pull request from Dependabot updates all of them
- **AND** that pull request changes files under `server/` only

### Requirement: CI builds the shell on every pull request that changes it

A CI job, the flake check, SHALL run `nix flake check` on every pull request that changes one of these files:

- `flake.nix` or `flake.lock`;
- any file under `nix/`;
- `server/rust-toolchain.toml`;
- the workflow file that defines the flake check.

The flake check SHALL fail when the shell cannot be built.

#### Scenario: A pull request breaks the shell

- **WHEN** a pull request changes `flake.nix` so that the development shell cannot be built
- **THEN** the flake check on that pull request fails

#### Scenario: A pull request leaves the shell working

- **WHEN** a pull request changes `flake.lock` and the development shell still builds
- **THEN** the flake check on that pull request passes

#### Scenario: A pull request does not touch the shell

- **WHEN** a pull request changes none of the files in this requirement's list, including the flake check's own workflow file
- **THEN** GitHub does not run the flake check on that pull request
