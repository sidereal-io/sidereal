# dev-commands Specification

## Purpose

The tasks in `mise.toml` run the server, the web shell and the database fixture for development, and the gate: the checks every pull request must pass.

## Requirements

### Requirement: The web shell runs on its own

`just web` SHALL install the web shell's dependencies from `web/pnpm-lock.yaml`, then start the web shell's dev server. It SHALL NOT change `web/pnpm-lock.yaml`. It SHALL stop with an error when the lockfile does not match `web/package.json`.

#### Scenario: Dependencies are not installed yet

- **WHEN** `web/node_modules` does not exist and a contributor runs `just web`
- **THEN** the web shell's dev server starts on port 5173

#### Scenario: The lockfile is out of date

- **WHEN** a contributor adds a dependency to `web/package.json` without updating `web/pnpm-lock.yaml`, then runs `just web`
- **THEN** the command exits with a non-zero status before the dev server starts
- **AND** `git diff --exit-code web/pnpm-lock.yaml` exits with status 0

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

### Requirement: Each web check has its own script

`web/package.json` SHALL give each check its own script: `typecheck`, `lint`, `format:check`, `design:lint`, `tokens:check`, and `test`. A contributor can then run one check alone.

#### Scenario: The six scripts exist

- **WHEN** a reviewer runs `jq -r '.scripts | keys[]' web/package.json`
- **THEN** the output includes `typecheck`, `lint`, `format:check`, `design:lint`, `tokens:check`, and `test`

### Requirement: One command runs the server and the web shell

`just dev` SHALL start the server and the web shell together against a prepared database configured through `DATABASE_URL`. One interrupt, such as Ctrl-C, SHALL stop both. When either one exits, `just dev` SHALL stop the other and exit. It SHALL exit with a non-zero status when either one failed. `just dev` SHALL NOT clear the terminal, so earlier output, such as a cargo error, stays visible.

#### Scenario: A fresh clone runs the stack

- **WHEN** a contributor clones the repo and prepares PostgreSQL using the documented root fixture recipe
- **AND** configures `DATABASE_URL` and runs `just dev` inside the development shell
- **AND** opens `http://localhost:5173` once the server has built
- **THEN** the page shows `healthy`
- **AND** the repo root still has no `node_modules` folder

#### Scenario: Ctrl-C stops the whole stack

- **GIVEN** a prepared compatible database and configured `DATABASE_URL`
- **WHEN** `just dev` is running and the contributor presses Ctrl-C
- **THEN** `just` exits
- **AND** within 5 seconds nothing listens on port 5000 or port 5173

#### Scenario: The terminal keeps earlier output

- **GIVEN** a prepared compatible database and configured `DATABASE_URL`
- **WHEN** a tester runs `script -qc 'just dev' dev.log` inside the development shell on Linux
- **AND** stops it with Ctrl-C after the web shell's dev server listens on port 5173
- **THEN** `dev.log` contains none of these escape sequences: `ESC [ 2 J`, `ESC [ 0 J`, `ESC [ J`, or `ESC c`

#### Scenario: The web shell fails to start

- **GIVEN** a prepared compatible database and configured `DATABASE_URL`
- **WHEN** another process listens on port 5173, and a contributor runs `just dev`
- **THEN** `just dev` exits with a non-zero status within 10 seconds
- **AND** nothing that `just dev` started is still running

### Requirement: The justfile has no v0.10.x recipes

The root `justfile` SHALL define no recipe that runs or checks the v0.10.x app. No recipe SHALL belong to a group named `v2` or `v0.10.x`.

#### Scenario: A contributor runs an old v0 recipe

- **WHEN** a contributor runs `just v0-dev` or `just v0-frontend`
- **THEN** the command exits with a non-zero status

#### Scenario: A reviewer reads the recipe groups

- **WHEN** a reviewer runs `just --dump --dump-format json` and reads each recipe's `group` attribute
- **THEN** no recipe is in group `v2` or group `v0.10.x`

### Requirement: Root recipes report fixture connection settings

`just db-url` SHALL report the runtime fixture connection URL for the selected worktree project and loopback port.
`just db-test-url` SHALL report its separate test-role URL.
Both recipes SHALL use only fixture credentials and SHALL NOT read or print production `DATABASE_URL`.
Operator guidance SHALL show exporting these values before running development and test commands.

#### Scenario: Fixture URLs match the selected port
- **WHEN** a contributor selects a fixture port and runs `just db-url` and `just db-test-url`
- **THEN** both URLs use that port, their distinct documented roles, and explicit `sslmode=disable`

#### Scenario: Production configuration stays private
- **WHEN** a contributor runs either URL recipe with a distinctive production `DATABASE_URL` in the environment
- **THEN** the output contains only the fixture URL and no part of that production connection string

### Requirement: One recipe builds and verifies the Compose instance

`just instance-verify` SHALL build the app image and start a disposable instance from the root `compose.yml`.
It SHALL then run these checks, and SHALL exit with a non-zero status when any one fails:

- the `db` image in `compose.yml` equals the pin in `server/postgres-image.env`;
- the home page, `/healthz`, and `/readyz` return their documented answers;
- the `app` container runs as user ID 10001, and the `db` container publishes no host port.

#### Scenario: The instance is sound

- **GIVEN** a running Docker daemon
- **WHEN** a contributor runs `just instance-verify` on an unchanged checkout
- **THEN** the command exits with status 0
- **AND** `docker ps -a` and `docker volume ls` list nothing that belongs to the recipe's Compose project

#### Scenario: The web shell is missing from the image

- **WHEN** a tester removes the step that copies the built web shell into the image, then runs `just instance-verify`
- **THEN** the command exits with a non-zero status within 240 seconds of the image build ending
- **AND** `docker ps -a` and `docker volume ls` list nothing that belongs to the recipe's Compose project

#### Scenario: The PostgreSQL pins differ

- **WHEN** a tester changes the `db` image tag in `compose.yml`, then runs `just instance-verify`
- **THEN** the command exits with a non-zero status before it builds the image

#### Scenario: Docker is not running

- **WHEN** a contributor runs `just instance-verify` with no reachable Docker daemon
- **THEN** the command exits with a non-zero status, and its output names Docker

### Requirement: The instance check covers the password, storage, retention and logs

`just instance-verify` SHALL also run these checks, and SHALL exit with a non-zero status when any one fails:

- Compose refuses to start without `POSTGRES_PASSWORD`;
- the app user can write to the storage volume;
- the `db` container keeps its data in the project's named database volume;
- the migration records are identical after `docker compose down` and a new start;
- the container logs do not contain a generated password, after a successful start and after a failed database login.

#### Scenario: The app logs its connection URL

- **WHEN** a tester changes the app so that it writes its `DATABASE_URL` to the log when it starts, then runs `just instance-verify`
- **THEN** the command exits with a non-zero status

### Requirement: The volume check runs before any container is removed

`just instance-verify` SHALL run the volume check before it removes any container. A failed volume check then leaves no unnamed volume behind.

#### Scenario: The database volume is not kept

- **GIVEN** a saved copy of the output of `docker volume ls -q`
- **WHEN** a tester removes the `db` service's volume mount from `compose.yml`, then runs `just instance-verify`
- **THEN** the command exits with a non-zero status
- **AND** `docker ps -a` lists nothing that belongs to the recipe's Compose project
- **AND** `docker volume ls -q` lists no volume that the saved copy lacks

### Requirement: The instance check keeps to its own project and settings

`just instance-verify` SHALL use its own Compose project name, its own image tag, its own generated password, and its own host port.
It SHALL NOT read or change the `.env` file at the repo root. `POSTGRES_PASSWORD` and `SIDEREAL_PORT` values in the caller's shell SHALL NOT reach the disposable instance.

#### Scenario: The caller's shell holds conflicting settings

- **GIVEN** a shell that exports `POSTGRES_PASSWORD=shellsecret` and `SIDEREAL_PORT=5000`
- **WHEN** a contributor runs `just instance-verify` and reads the disposable `app` container's environment with `docker inspect`
- **THEN** its `DATABASE_URL` does not contain `shellsecret`
- **AND** the disposable instance publishes a host port other than 5000

#### Scenario: Sam's own instance is running

- **GIVEN** an instance that Sam started from the repo root with their own `.env` file
- **WHEN** a contributor runs `just instance-verify`
- **THEN** the command exits with status 0
- **AND** Sam's containers keep their container IDs, and Sam's volumes still exist
- **AND** the image ID that `docker compose images -q app` prints for Sam's project is unchanged
- **AND** `git status --porcelain` and the contents of `.env` are unchanged

### Requirement: Each start of the instance check has 180 seconds

`just instance-verify` SHALL give each start at most 180 seconds to become healthy, after the image is built. It SHALL measure that time itself, stop a start that takes longer, and fail.

#### Scenario: The database never becomes healthy

- **WHEN** a tester changes the `db` service's health check command in `compose.yml` to `false`, then runs `just instance-verify`
- **THEN** the command exits with a non-zero status within 240 seconds of the image build ending
- **AND** `docker ps -a` and `docker volume ls` list nothing that belongs to the recipe's Compose project

### Requirement: The instance check stops before it starts on a bad setup

`just instance-verify` SHALL stop before it builds anything when Compose is older than release 2.20. Its output SHALL name the release it needs.
The recipe SHALL stop before it starts anything when its Compose project name already owns a container or a volume that this checkout did not create. It SHALL remove nothing in that case.

#### Scenario: Compose is too old

- **WHEN** a contributor runs `just instance-verify` with a Compose release older than 2.20
- **THEN** the command exits with a non-zero status, and its output names release 2.20
- **AND** `docker images` lists no image that belongs to the recipe's Compose project

#### Scenario: Another checkout owns the project name

- **GIVEN** a container that carries the recipe's Compose project label, created outside this checkout
- **WHEN** a contributor runs `just instance-verify`
- **THEN** the command exits with a non-zero status, and its output names the project
- **AND** that container still exists

### Requirement: One instance check runs per checkout at a time

Only one run of `just instance-verify` SHALL work on a checkout at a time. A second run in the same checkout SHALL exit with a non-zero status and change nothing.

#### Scenario: A second run starts in the same checkout

- **GIVEN** a run of `just instance-verify` that has started its disposable instance
- **WHEN** a contributor starts a second `just instance-verify` in the same checkout
- **THEN** the second command exits with a non-zero status within 10 seconds, and its output says a run is in progress
- **AND** the first run exits with status 0

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

