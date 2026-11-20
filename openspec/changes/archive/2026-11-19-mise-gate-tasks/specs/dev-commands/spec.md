## ADDED Requirements

### Requirement: mise tasks hold the gate

`mise.toml` SHALL define the gate as five tasks: `check-compose`, `check-openspec`, `check-server`, `check-web` and `check`. A contributor runs each one as `mise run <task>`. The root `justfile` SHALL define none of these five names, so the gate is defined in one place.

#### Scenario: The five tasks exist

- **WHEN** a reviewer runs `mise tasks ls --no-header` at the repo root
- **THEN** the output includes `check-compose`, `check-openspec`, `check-server`, `check-web` and `check`

#### Scenario: The justfile has no gate recipe

- **WHEN** a reviewer runs `just --summary` at the repo root
- **THEN** the output names none of `check`, `check-compose`, `check-openspec`, `check-server` and `check-web`

#### Scenario: The old command is gone

- **WHEN** a contributor runs `just check` at the repo root
- **THEN** the command exits with a non-zero status

### Requirement: The gate runs its parts in order and stops at the first failure

`mise run check` SHALL run `check-compose`, then `check-openspec`, then `check-server`, then `check-web`, one at a time. When one part fails, `mise run check` SHALL start no later part, and SHALL exit with a non-zero status. When every part passes, it SHALL exit with status 0.

#### Scenario: The full gate order

- **WHEN** a reviewer runs `mise tasks info check --json | jq -r '.run[].task'` at the repo root
- **THEN** the output is exactly four lines, in this order: `check-compose`, `check-openspec`, `check-server`, `check-web`
- **AND** `mise tasks info check --json | jq '.depends | length'` prints `0`

#### Scenario: An early part fails

- **WHEN** a tester changes `scripts/compose-config-check.sh` so that it exits with status 1 as its first command, then runs `mise run check`
- **THEN** the command exits with a non-zero status
- **AND** its output contains no line that starts with `[check-openspec]`, `[check-server]` or `[check-web]`

#### Scenario: Every part passes

- **GIVEN** a prepared PostgreSQL fixture, with `TEST_DATABASE_URL` set to the URL `just db-test-url` prints
- **WHEN** a contributor runs `mise run check` at the repo root on an unchanged checkout
- **THEN** the command exits with status 0

## MODIFIED Requirements

### Requirement: The web gate fails on any web check error

`mise run check-web` SHALL install the web shell's dependencies from `web/pnpm-lock.yaml`, then run six checks on `web/`: a type check, a lint, a format check, a lint of `DESIGN.md`, a token drift check, and the unit tests.

`mise run check-web` SHALL exit with a non-zero status when any check fails, or when the lockfile does not match `web/package.json`. It SHALL NOT change any file that git tracks.

#### Scenario: The web shell is clean

- **WHEN** a contributor runs `mise run check-web` at the repo root on an unchanged checkout
- **THEN** the command exits with status 0
- **AND** `git status --porcelain` prints nothing afterwards

#### Scenario: Dependencies are not installed yet

- **WHEN** `web/node_modules` does not exist and a contributor runs `mise run check-web`
- **THEN** the command installs the dependencies, runs the six checks, and exits with status 0

#### Scenario: The lockfile is out of date

- **WHEN** a tester adds a dependency to `web/package.json` without updating `web/pnpm-lock.yaml`, then runs `mise run check-web`
- **THEN** the command exits with a non-zero status
- **AND** `git diff --exit-code web/pnpm-lock.yaml` exits with status 0

#### Scenario: The full gate stops on a web error

- **WHEN** a tester makes any change above that fails `mise run check-web`, then runs `mise run check`
- **THEN** `mise run check` exits with a non-zero status

### Requirement: The web gate checks types, lint rules and formatting

The type check SHALL cover every TypeScript file in `web/`, including config files, scripts, and test files.
The lint SHALL include rules which read TypeScript types, and rules for React components and hooks.
The format check SHALL fail when any file differs from the formatter's output.

#### Scenario: A type error in app code

- **WHEN** a tester assigns a number to a `string` variable in a file under `web/src/`, then runs `mise run check-web`
- **THEN** the command exits with a non-zero status

#### Scenario: A type error in a config file

- **WHEN** a tester assigns a number to a `string` variable in `web/vite.config.ts`, then runs `mise run check-web`
- **THEN** the command exits with a non-zero status

#### Scenario: A type error in a script

- **WHEN** a tester assigns a number to a `string` variable in a file under `web/scripts/`, then runs `mise run check-web`
- **THEN** the command exits with a non-zero status

#### Scenario: A promise is left unhandled

- **WHEN** a tester adds, in a file under `web/src/`, a call to an async function whose result is not awaited, returned, or marked with `void`
- **AND** runs `mise run check-web`
- **THEN** the command exits with a non-zero status

#### Scenario: A hook is called conditionally

- **WHEN** a tester moves a `useState` call in a component under `web/src/` inside an `if` block, then runs `mise run check-web`
- **THEN** the command exits with a non-zero status

#### Scenario: A file is not formatted

- **WHEN** a tester adds a blank line with trailing spaces to a file under `web/src/`, then runs `mise run check-web`
- **THEN** the command exits with a non-zero status
- **AND** the file still has the trailing spaces

### Requirement: The web gate checks DESIGN.md, the tokens and the unit tests

The lint of `DESIGN.md` SHALL use the DESIGN.md specification's linter.
The token drift check SHALL fail when `web/src/styles/tokens.css` does not match `DESIGN.md`.
The unit tests SHALL include the contrast test for `DESIGN.md`.

#### Scenario: A unit test fails

- **WHEN** a tester changes a unit test's expected state from `healthy` to `checking`, then runs `mise run check-web`
- **THEN** the command exits with a non-zero status

### Requirement: The instance check removes what it made and stays out of the gate

`just instance-verify` SHALL remove its containers, network, volumes, and image when it ends, whether it passes or fails. It SHALL remove nothing that belongs to another Compose project.
`mise run check` SHALL NOT run `just instance-verify`.

#### Scenario: The gate does not build the image

- **WHEN** a reviewer runs `mise tasks info check --json | jq -r '.run[].task'` at the repo root
- **THEN** the output is exactly four lines, in this order: `check-compose`, `check-openspec`, `check-server`, `check-web`
- **AND** `mise tasks info check --json` does not contain `instance-verify`

### Requirement: The OpenSpec gate fails on any validation error or warning

`mise run check-openspec` SHALL validate every spec under `openspec/specs/` and every active change under `openspec/changes/` in strict mode. It SHALL exit with a non-zero status when validation reports an error or a warning. A validation note that is neither an error nor a warning SHALL NOT fail it.

`mise run check` SHALL run `check-openspec` after `check-compose` and before `check-server`.

#### Scenario: The specs and changes are valid

- **WHEN** a contributor runs `mise run check-openspec` at the repo root on an unchanged checkout
- **THEN** the command exits with status 0
- **AND** `git status --porcelain` prints nothing

#### Scenario: A requirement has no scenario

- **WHEN** a tester deletes every scenario from one requirement in a spec under `openspec/specs/`, then runs `mise run check-openspec`
- **THEN** the command exits with a non-zero status, and its output names that spec

#### Scenario: A spec has only a warning

- **WHEN** a tester shortens one spec's Purpose section under `openspec/specs/` to the word `Short.`, then runs `mise run check-openspec`
- **THEN** the command exits with a non-zero status, and its output names that spec

#### Scenario: An active change is invalid

- **WHEN** a tester adds a requirement with no scenario to a delta spec in an active change under `openspec/changes/`, then runs `mise run check-openspec`
- **THEN** the command exits with a non-zero status, and its output names that change

#### Scenario: The full gate stops on an OpenSpec error

- **WHEN** a tester makes any change above that fails `mise run check-openspec`, then runs `mise run check`
- **THEN** `mise run check` exits with a non-zero status

#### Scenario: The full gate order

- **WHEN** a reviewer runs `mise tasks info check --json | jq -r '.run[].task'` at the repo root
- **THEN** `check-openspec` is the second of the four lines, after `check-compose` and before `check-server`

### Requirement: The OpenSpec gate runs unattended

`mise run check-openspec` SHALL NOT wait for input. It SHALL run the `openspec` CLI with usage telemetry turned off, and SHALL NOT change any file that git tracks.

#### Scenario: No terminal is attached

- **WHEN** a contributor runs `mise run check-openspec < /dev/null` at the repo root on an unchanged checkout
- **THEN** the command exits with status 0 without waiting for input
