<!-- Plain language: Name the actor, condition and observable result. Explain necessary domain terms using the agreed glossary. Keep requirement and scenario headings and WHEN/THEN syntax exactly as required. -->

## ADDED Requirements

### Requirement: The mise check fails when the lock misses a supported platform

The mise check SHALL fail when a `[[tools.*]]` entry in `mise.lock` has no `platforms.linux-x64`, `platforms.linux-arm64` or `platforms.macos-arm64` table. The one exception SHALL be `[[tools.rust]]`, which records the `stable` channel and has no platform tables. The check's log SHALL name each tool that lacks a table and each platform it lacks. The check SHALL NOT require any other platform.

#### Scenario: A tool lacks a supported platform

- **WHEN** a pull request removes the `[tools.just."platforms.macos-arm64"]` table from `mise.lock`
- **THEN** the mise check on that pull request fails
- **AND** its log names the tool `just` and the platform `macos-arm64`

### Requirement: Contributors can run the lock platform check by hand

`scripts/check-mise-lock-platforms.sh`, run at the repo root, SHALL apply the same rule to `mise.lock` as the mise check. It SHALL exit with status 0 when every entry has its supported platforms, and with a non-zero status otherwise. Its output SHALL name each tool that lacks a table and each platform it lacks.

#### Scenario: The committed lock passes

- **WHEN** a contributor runs `scripts/check-mise-lock-platforms.sh` at the repo root on a clean checkout
- **THEN** the script exits with status 0

#### Scenario: Several tools lack platforms

- **WHEN** a tester removes the `[tools.gh."platforms.linux-arm64"]` and `[tools.pnpm."platforms.linux-x64"]` tables from `mise.lock`, then runs `scripts/check-mise-lock-platforms.sh` at the repo root
- **THEN** the script exits with a non-zero status
- **AND** its output names `gh` with `linux-arm64`, and `pnpm` with `linux-x64`

#### Scenario: A new tool has no platform tables

- **WHEN** a tester adds a `[[tools.example]]` entry with a `version` and no platform tables to `mise.lock`, then runs `scripts/check-mise-lock-platforms.sh` at the repo root
- **THEN** the script exits with a non-zero status
- **AND** its output names `example` with `linux-x64`, `linux-arm64` and `macos-arm64`

#### Scenario: Rust has no platform tables

- **WHEN** the `[[tools.rust]]` entry in `mise.lock` has no platform tables and every other entry has all three supported platforms
- **THEN** `scripts/check-mise-lock-platforms.sh` exits with status 0

#### Scenario: Windows is not required

- **WHEN** a tester removes every `windows-x64` table from `mise.lock`, then runs `scripts/check-mise-lock-platforms.sh` at the repo root
- **THEN** the script exits with status 0
