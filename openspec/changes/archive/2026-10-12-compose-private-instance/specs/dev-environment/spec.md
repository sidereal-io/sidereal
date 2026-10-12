## MODIFIED Requirements

### Requirement: No file names a Rust release

The repo SHALL name no Rust release number in its build files or its contributor docs. The only Rust version setting SHALL be the channel in `server/rust-toolchain.toml`. The app image SHALL name only Rust major version 1 in its base image, and SHALL still build.

#### Scenario: The Cargo manifest sets no minimum Rust version

- **WHEN** a reviewer runs `grep -rn 'rust-version' server --include=Cargo.toml`
- **THEN** the search finds no match

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

### Requirement: Contributors and CI name one Node version

The repo SHALL state Node major version 26 wherever it names a Node version for contributors. `.nvmrc` SHALL contain the major version only, so Nix and Node version managers agree. A CI workflow that sets up Node outside the development shell SHALL read `.nvmrc` rather than hold its own copy of the version, so CI can never drift from it. The app image SHALL build the web shell with a Node base image of the major version that `.nvmrc` names.

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
