## MODIFIED Requirements

### Requirement: The shell provides the pinned toolchain

The development shell SHALL provide these tools on `PATH`:

- Rust at a stable release, with `rustfmt` and `clippy`;
- Node at major version 26;
- pnpm at major version 12;
- `just`;
- the `openspec` CLI.

#### Scenario: A contributor checks tool versions in the shell

- **WHEN** a contributor runs `nix develop --command sh -c 'rustc --version; node --version; pnpm --version; just --version; openspec --version'` at the repo root
- **THEN** `rustc --version` output starts with `rustc 1.`
- **AND** the `rustc --version` output contains neither `beta` nor `nightly`
- **AND** `node --version` output starts with `v26.`
- **AND** `pnpm --version` output starts with `12.`
- **AND** the `just` and `openspec` commands each exit with status 0

#### Scenario: Formatting and lint tools come with Rust

- **WHEN** a contributor runs `nix develop --command sh -c 'cargo fmt --version && cargo clippy --version'`
- **THEN** both commands exit with status 0

### Requirement: The flake check reports the shell's tool versions

After the shell builds, the flake check SHALL run `rustc --version`, `openspec --version`, `node --version`, `pnpm --version`, and `just --version` inside the shell. A reviewer SHALL be able to read their output in the job log. The flake check SHALL fail when any of the five commands fails.

#### Scenario: A reviewer checks what a pin update changed

- **WHEN** the flake check passes on a pull request
- **THEN** the job log contains the output of `rustc --version`, `openspec --version`, `node --version`, `pnpm --version`, and `just --version`, each run inside the development shell

#### Scenario: A pin update breaks a tool

- **WHEN** `openspec --version` exits with a non-zero status inside the shell, and the other four commands succeed
- **THEN** the flake check fails
