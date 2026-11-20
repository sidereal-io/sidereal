## RENAMED Requirements

- FROM: `### Requirement: The repo checks pass inside the shell`
- TO: `### Requirement: The repo checks pass through mise`

## MODIFIED Requirements

### Requirement: Nix stays optional

The environment SHALL NOT require Nix for building or checking the repo. For a contributor with direnv but without Nix, entering the repo SHALL produce no error. It SHALL change no environment variables except direnv's own `DIRENV_*` variables and any that the contributor's `.envrc.local` sets. The repo SHALL keep the pin files that tools outside Nix read: `server/rust-toolchain.toml` for rustup, and `.nvmrc` for Node version managers.

#### Scenario: A contributor has direnv but not Nix

- **WHEN** a contributor without Nix and without `.envrc.local` runs `direnv allow`, then enters the repo directory
- **THEN** direnv reports no error
- **AND** `env`, with lines starting `DIRENV_` removed, shows the same output as before the contributor entered the directory

#### Scenario: A contributor without Nix runs the checks

- **GIVEN** a contributor with neither Nix nor mise, who installs the tools by hand as `CONTRIBUTING.md` says, and prepares the PostgreSQL fixture
- **WHEN** the contributor installs Rust through rustup and installs Node through a version manager reading `.nvmrc`
- **THEN** rustup selects the channel in `server/rust-toolchain.toml`
- **AND** the version manager selects Node major version 26
- **AND** each of the four gate commands that `CONTRIBUTING.md` lists, for Compose, OpenSpec, the server and the web shell, exits with status 0 when run in that order from the repo root

#### Scenario: The environment does not load application settings

- **WHEN** a contributor enters the repo directory with the development shell turned on
- **THEN** no variable defined only in `.env` is set in the contributor's shell

### Requirement: The repo checks pass through mise

The repo's gate SHALL pass when a contributor runs it through mise. The gate is `mise run check`: the Compose configuration checks, the OpenSpec checks, the server checks, and the web checks. The Nix development shell does not provide mise, so a contributor on the Nix route also runs the gate through mise.

#### Scenario: A contributor runs the gate in the shell

- **GIVEN** a prepared PostgreSQL fixture, with `TEST_DATABASE_URL` set to the URL `just db-test-url` prints
- **WHEN** a contributor runs `mise run check` at the repo root, on a commit whose `ci` workflow passes
- **THEN** the command exits with status 0
