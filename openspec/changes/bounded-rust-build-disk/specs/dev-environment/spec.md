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

- **WHEN** a contributor runs `cargo test --no-run -v -p sidereal-plugin-abi` in `server/` after `cargo clean -p sidereal-plugin-abi`
- **THEN** the compiler command for the `sidereal_plugin_abi` crate contains `-C debuginfo=line-tables-only`

### Requirement: Contributors can turn full debug information back on

`CONTRIBUTING.md` SHALL give one command line that builds with full debug information for the workspace crates and for every dependency, without editing a committed file. The command line SHALL work with and without mise.

#### Scenario: The documented command restores full debug information

- **WHEN** a contributor runs, in `server/`, the command line that `CONTRIBUTING.md` gives for full debug information, with `-v` added and `-p sidereal-plugin-abi` as the package, after `cargo clean -p sidereal-plugin-abi`
- **THEN** the compiler command for the `sidereal_plugin_abi` crate contains `-C debuginfo=2`
- **AND** `git status --porcelain` prints nothing
