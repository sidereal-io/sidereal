## Purpose

The development environment gives every contributor and AI agent the same pinned tools for this repo. It turns on automatically when Nix is installed. It stays optional, so contributors without Nix can still build and check the repo.

## ADDED Requirements

### Requirement: The shell provides the pinned toolchain

The development shell SHALL provide these tools on `PATH`:

- Rust at the exact channel named in `backend/rust-toolchain.toml`, with `rustfmt` and `clippy`;
- Node at major version 24;
- `just`;
- the `openspec` CLI.

#### Scenario: A contributor checks tool versions in the shell

- **WHEN** a contributor runs `nix develop --command sh -c 'rustc --version; node --version; just --version; openspec --version'` at the repo root
- **THEN** `rustc --version` reports the version in the `channel` field of `backend/rust-toolchain.toml`
- **AND** `node --version` output starts with `v24.`
- **AND** the `just` and `openspec` commands each exit with status 0

#### Scenario: Formatting and lint tools come with Rust

- **WHEN** a contributor runs `nix develop --command sh -c 'cargo fmt --version && cargo clippy --version'`
- **THEN** both commands exit with status 0

### Requirement: The Rust version has one source of truth

The development shell SHALL take the Rust version and components from `backend/rust-toolchain.toml` only. Changing the Rust version SHALL NOT require editing any other file.

#### Scenario: A maintainer bumps the Rust version

- **WHEN** a maintainer changes `channel` in `backend/rust-toolchain.toml` to another stable version, and changes no other file
- **THEN** `nix develop --command rustc --version` reports the new version

### Requirement: The lock file decides every tool version

The development shell SHALL resolve every tool from inputs pinned in the committed `flake.lock`. The same commit SHALL produce the same shell on every machine of the same system type.

#### Scenario: Two clones of one commit agree

- **WHEN** a tester clones the same commit into two different directories on one machine
- **AND** runs `nix eval --raw .#devShells.<system>.default.drvPath` in each clone
- **THEN** both commands print an identical derivation path

### Requirement: Remote code is verified before it runs

The environment SHALL verify every piece of remote code it fetches against a hash committed to the repo. This covers flake inputs and any script that `.envrc` loads. The environment SHALL refuse to use content whose hash does not match.

#### Scenario: A fetched script does not match its hash

- **WHEN** `.envrc` fetches a remote script whose content differs from the hash committed in `.envrc`
- **THEN** direnv stops with an error and does not run the script

#### Scenario: Every flake input is locked

- **WHEN** a reviewer runs `nix flake metadata --json` at the repo root
- **THEN** every input in the output has a `locked` entry with a `narHash`

### Requirement: The shell turns on when you enter the repo

For a contributor with Nix and direnv, entering the repo directory SHALL turn the development shell on after a one-time `direnv allow`. Leaving the directory SHALL restore the contributor's previous environment. The shell SHALL reload when a file that defines it changes.

#### Scenario: A contributor enters and leaves the repo

- **WHEN** a contributor with Nix and direnv has run `direnv allow` once, then changes into the repo directory
- **THEN** `rustc --version` reports the pinned version without any further command
- **AND** after the contributor changes to a directory outside the repo, `rustc --version` reports the same result as before they entered it

#### Scenario: The Rust pin changes while the shell is active

- **WHEN** `backend/rust-toolchain.toml` or any file under `nix/` changes while the contributor is inside the repo directory
- **THEN** direnv reloads the development shell at the contributor's next prompt

### Requirement: Contributors can add their own direnv settings

`.envrc` SHALL load a file named `.envrc.local` when it exists, whether or not Nix is installed. Git SHALL ignore `.envrc.local`, so each contributor keeps their own settings out of commits.

#### Scenario: A contributor adds a personal variable

- **WHEN** a contributor writes `export SIDEREAL_TEST_VAR=1` in `.envrc.local` and enters the repo directory
- **THEN** `SIDEREAL_TEST_VAR` equals `1` in the contributor's shell
- **AND** the result is the same with Nix installed and without it

#### Scenario: The personal file stays out of git

- **WHEN** a contributor creates `.envrc.local` and runs `git status --porcelain`
- **THEN** the output does not list `.envrc.local`

### Requirement: Nix stays optional

The environment SHALL NOT require Nix for building or checking the repo. For a contributor with direnv but without Nix, entering the repo SHALL produce no error. It SHALL change no environment variables except direnv's own `DIRENV_*` variables and any that the contributor's `.envrc.local` sets. The repo SHALL keep the pin files that tools outside Nix read: `backend/rust-toolchain.toml` for rustup, and `.nvmrc` for Node version managers.

#### Scenario: A contributor has direnv but not Nix

- **WHEN** a contributor without Nix and without `.envrc.local` runs `direnv allow`, then enters the repo directory
- **THEN** direnv reports no error
- **AND** `env`, with lines starting `DIRENV_` removed, shows the same output as before the contributor entered the directory

#### Scenario: A contributor without Nix runs the checks

- **WHEN** a contributor without Nix installs Rust through rustup and installs Node through a version manager reading `.nvmrc`
- **THEN** rustup selects the channel in `backend/rust-toolchain.toml`
- **AND** the version manager selects Node major version 24
- **AND** `just check` exits with status 0, given that `just` is installed

#### Scenario: The environment does not load application settings

- **WHEN** a contributor enters the repo directory with the development shell turned on
- **THEN** no variable defined only in `.env` is set in the contributor's shell

### Requirement: The repo checks pass inside the shell

The repo's existing gate SHALL pass inside the development shell. The gate is `just check`: `cargo fmt --check`, clippy with warnings denied, `cargo test`, and the dependency-direction lint.

#### Scenario: A contributor runs the gate in the shell

- **WHEN** a contributor runs `nix develop --command just check` on a commit whose checks pass outside Nix
- **THEN** the command exits with status 0

### Requirement: The flake check builds the shell

`nix flake check` SHALL build the development shell. It SHALL fail when the shell cannot be built.

#### Scenario: The shell definition is broken

- **WHEN** a change makes the development shell impossible to build, and a reviewer runs `nix flake check`
- **THEN** `nix flake check` exits with a non-zero status

#### Scenario: The shell definition is sound

- **WHEN** a reviewer runs `nix flake check` on a commit whose shell builds
- **THEN** `nix flake check` exits with status 0

### Requirement: The shell is defined for the supported systems

The flake SHALL define a development shell for `x86_64-linux`, `aarch64-linux` and `aarch64-darwin`.

#### Scenario: A contributor lists the development shells

- **WHEN** a contributor runs `nix flake show --json --all-systems`
- **THEN** the output contains `devShells.x86_64-linux.default`, `devShells.aarch64-linux.default` and `devShells.aarch64-darwin.default`

### Requirement: Every document names the same Node version

The repo SHALL state Node major version 24 wherever it names a Node version for contributors. `.nvmrc` SHALL contain the major version only, so Nix, CI and Node version managers agree.

#### Scenario: A reviewer checks the documented Node version

- **WHEN** a reviewer reads `.nvmrc`
- **THEN** the file contains `24` and nothing else except a trailing newline

#### Scenario: No outdated Node version is left in the docs

- **WHEN** a reviewer searches `AGENTS.md`, `backend/README.md` and `CONTRIBUTING.md` for "Node 20" and "Node.js 20"
- **THEN** the search finds no match
