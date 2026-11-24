## ADDED Requirements

### Requirement: Development builds keep debug information light

`server/Cargo.toml` SHALL set the `dev` profile to build the workspace's own crates with line tables only, and every dependency with no debug information. The `test` profile SHALL inherit these settings from `dev`, so `cargo build`, `cargo run`, `cargo test` and `cargo clippy` all use them. `server/Cargo.toml` SHALL NOT change the `release` profile, so the release build keeps Cargo's default of no debug information.

#### Scenario: The dev profile sets line tables only for the workspace

- **WHEN** a reviewer reads the `[profile.dev]` table in `server/Cargo.toml`
- **THEN** its `debug` key is `"line-tables-only"`

#### Scenario: The dev profile drops debug information for dependencies

- **WHEN** a reviewer reads the `[profile.dev.package."*"]` table in `server/Cargo.toml`
- **THEN** its `debug` key is `false`

#### Scenario: Only the dev profile is set

- **WHEN** a reviewer runs `grep -n '^\[profile' server/Cargo.toml`
- **THEN** the output is exactly two lines, naming `[profile.dev]` and `[profile.dev.package."*"]`

#### Scenario: A test build uses the light settings

- **WHEN** a contributor runs `cargo test --no-run -v -p sidereal-plugin-abi` in `server/`, with `CARGO_TARGET_DIR` set to a new empty folder
- **THEN** the command succeeds
- **AND** the compiler command for the `sidereal_plugin_abi` crate contains `-C debuginfo=line-tables-only`

#### Scenario: A dependency builds without debug information

- **WHEN** a contributor runs `cargo build -v -p libc` in `server/`, with `CARGO_TARGET_DIR` set to a new empty folder
- **THEN** the command succeeds
- **AND** the compiler command for the `libc` crate contains no `-C debuginfo=` option
- **AND** `-C strip=debuginfo`, which Cargo adds when debug information is off, does not count as a `-C debuginfo=` option

### Requirement: Contributors can turn full debug information back on

`CONTRIBUTING.md` SHALL give one command line that builds with full debug information for the workspace crates and for every dependency, without editing a committed file. The command line SHALL work with and without mise.

#### Scenario: The documented command restores full debug information

- **WHEN** a contributor runs, in `server/` with `CARGO_TARGET_DIR` set to a new empty folder, the command line that `CONTRIBUTING.md` gives for full debug information, once with `build -v -p sidereal-plugin-abi` as the command and once with `build -v -p libc`
- **THEN** both commands succeed
- **AND** the compiler commands for the `sidereal_plugin_abi` crate and the `libc` crate both contain `-C debuginfo=2`
- **AND** `git status --porcelain` prints the same before and after the command
