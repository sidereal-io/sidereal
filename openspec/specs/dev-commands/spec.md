# dev-commands Specification

## Purpose

The root `justfile` is the one place to run the server and the web shell for development, and to run the checks every pull request must pass.

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

`just check-web` SHALL install the web shell's dependencies from `web/pnpm-lock.yaml`, then run six checks on `web/`:

- a type check of every TypeScript file in `web/`, including config files, scripts, and test files;
- a lint that includes rules which read TypeScript types, and rules for React components and hooks;
- a format check, which fails when any file differs from the formatter's output;
- a lint of `DESIGN.md` with the DESIGN.md specification's linter;
- a token drift check, which fails when `web/src/styles/tokens.css` does not match `DESIGN.md`;
- the unit tests, including the contrast test for `DESIGN.md`.

`web/package.json` SHALL give each check its own script: `typecheck`, `lint`, `format:check`, `design:lint`, `tokens:check`, and `test`. A contributor can then run one check alone.

`just check-web` SHALL exit with a non-zero status when any check fails, or when the lockfile does not match `web/package.json`. It SHALL NOT change any file that git tracks.

#### Scenario: The web shell is clean

- **WHEN** a contributor runs `just check-web` on an unchanged checkout inside the development shell
- **THEN** the command exits with status 0
- **AND** `git status --porcelain` prints nothing afterwards

#### Scenario: Dependencies are not installed yet

- **WHEN** `web/node_modules` does not exist and a contributor runs `just check-web`
- **THEN** the command installs the dependencies, runs the six checks, and exits with status 0

#### Scenario: A type error in app code

- **WHEN** a tester assigns a number to a `string` variable in a file under `web/src/`, then runs `just check-web`
- **THEN** the command exits with a non-zero status

#### Scenario: A type error in a config file

- **WHEN** a tester assigns a number to a `string` variable in `web/vite.config.ts`, then runs `just check-web`
- **THEN** the command exits with a non-zero status

#### Scenario: A type error in a script

- **WHEN** a tester assigns a number to a `string` variable in a file under `web/scripts/`, then runs `just check-web`
- **THEN** the command exits with a non-zero status

#### Scenario: A promise is left unhandled

- **WHEN** a tester adds, in a file under `web/src/`, a call to an async function whose result is not awaited, returned, or marked with `void`
- **AND** runs `just check-web`
- **THEN** the command exits with a non-zero status

#### Scenario: A hook is called conditionally

- **WHEN** a tester moves a `useState` call in a component under `web/src/` inside an `if` block, then runs `just check-web`
- **THEN** the command exits with a non-zero status

#### Scenario: A file is not formatted

- **WHEN** a tester adds a blank line with trailing spaces to a file under `web/src/`, then runs `just check-web`
- **THEN** the command exits with a non-zero status
- **AND** the file still has the trailing spaces

#### Scenario: A unit test fails

- **WHEN** a tester changes a unit test's expected state from `healthy` to `checking`, then runs `just check-web`
- **THEN** the command exits with a non-zero status

#### Scenario: The lockfile is out of date

- **WHEN** a tester adds a dependency to `web/package.json` without updating `web/pnpm-lock.yaml`, then runs `just check-web`
- **THEN** the command exits with a non-zero status
- **AND** `git diff --exit-code web/pnpm-lock.yaml` exits with status 0

#### Scenario: The full gate stops on a web error

- **WHEN** a tester makes any change above that fails `just check-web`, then runs `just check`
- **THEN** `just check` exits with a non-zero status

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

- Compose refuses to start without `POSTGRES_PASSWORD`;
- the `db` image in `compose.yml` equals the pin in `server/postgres-image.env`;
- the home page, `/healthz`, and `/readyz` return their documented answers;
- the `app` container runs as user ID 10001, and the `db` container publishes no host port;
- the app user can write to the storage volume;
- the `db` container keeps its data in the project's named database volume;
- the migration records are identical after `docker compose down` and a new start;
- the container logs do not contain a generated password, after a successful start and after a failed database login.

The recipe SHALL run the volume check before it removes any container. A failed volume check then leaves no unnamed volume behind.
The recipe SHALL use its own Compose project name, its own image tag, its own generated password, and its own host port.
It SHALL NOT read or change the `.env` file at the repo root. `POSTGRES_PASSWORD` and `SIDEREAL_PORT` values in the caller's shell SHALL NOT reach the disposable instance.
The recipe SHALL give each start at most 180 seconds to become healthy, after the image is built. It SHALL measure that time itself, stop a start that takes longer, and fail.
The recipe SHALL stop before it builds anything when Compose is older than release 2.20. Its output SHALL name the release it needs.
The recipe SHALL stop before it starts anything when its Compose project name already owns a container or a volume that this checkout did not create. It SHALL remove nothing in that case.
Only one run SHALL work on a checkout at a time. A second run in the same checkout SHALL exit with a non-zero status and change nothing.
The recipe SHALL remove its containers, network, volumes, and image when it ends, whether it passes or fails. It SHALL remove nothing that belongs to another Compose project.
`just check` SHALL NOT run `just instance-verify`.

#### Scenario: The instance is sound

- **GIVEN** a running Docker daemon
- **WHEN** a contributor runs `just instance-verify` on an unchanged checkout
- **THEN** the command exits with status 0
- **AND** `docker ps -a` and `docker volume ls` list nothing that belongs to the recipe's Compose project

#### Scenario: The database volume is not kept

- **GIVEN** a saved copy of the output of `docker volume ls -q`
- **WHEN** a tester removes the `db` service's volume mount from `compose.yml`, then runs `just instance-verify`
- **THEN** the command exits with a non-zero status
- **AND** `docker ps -a` lists nothing that belongs to the recipe's Compose project
- **AND** `docker volume ls -q` lists no volume that the saved copy lacks

#### Scenario: The web shell is missing from the image

- **WHEN** a tester removes the step that copies the built web shell into the image, then runs `just instance-verify`
- **THEN** the command exits with a non-zero status within 240 seconds of the image build ending
- **AND** `docker ps -a` and `docker volume ls` list nothing that belongs to the recipe's Compose project

#### Scenario: The database never becomes healthy

- **WHEN** a tester changes the `db` service's health check command in `compose.yml` to `false`, then runs `just instance-verify`
- **THEN** the command exits with a non-zero status within 240 seconds of the image build ending
- **AND** `docker ps -a` and `docker volume ls` list nothing that belongs to the recipe's Compose project

#### Scenario: A second run starts in the same checkout

- **GIVEN** a run of `just instance-verify` that has started its disposable instance
- **WHEN** a contributor starts a second `just instance-verify` in the same checkout
- **THEN** the second command exits with a non-zero status within 10 seconds, and its output says a run is in progress
- **AND** the first run exits with status 0

#### Scenario: Another checkout owns the project name

- **GIVEN** a container that carries the recipe's Compose project label, created outside this checkout
- **WHEN** a contributor runs `just instance-verify`
- **THEN** the command exits with a non-zero status, and its output names the project
- **AND** that container still exists

#### Scenario: Compose is too old

- **WHEN** a contributor runs `just instance-verify` with a Compose release older than 2.20
- **THEN** the command exits with a non-zero status, and its output names release 2.20
- **AND** `docker images` lists no image that belongs to the recipe's Compose project

#### Scenario: The caller's shell holds conflicting settings

- **GIVEN** a shell that exports `POSTGRES_PASSWORD=shellsecret` and `SIDEREAL_PORT=5000`
- **WHEN** a contributor runs `just instance-verify` and reads the disposable `app` container's environment with `docker inspect`
- **THEN** its `DATABASE_URL` does not contain `shellsecret`
- **AND** the disposable instance publishes a host port other than 5000

#### Scenario: The PostgreSQL pins differ

- **WHEN** a tester changes the `db` image tag in `compose.yml`, then runs `just instance-verify`
- **THEN** the command exits with a non-zero status before it builds the image

#### Scenario: Sam's own instance is running

- **GIVEN** an instance that Sam started from the repo root with their own `.env` file
- **WHEN** a contributor runs `just instance-verify`
- **THEN** the command exits with status 0
- **AND** Sam's containers keep their container IDs, and Sam's volumes still exist
- **AND** the image ID that `docker compose images -q app` prints for Sam's project is unchanged
- **AND** `git status --porcelain` and the contents of `.env` are unchanged

#### Scenario: Docker is not running

- **WHEN** a contributor runs `just instance-verify` with no reachable Docker daemon
- **THEN** the command exits with a non-zero status, and its output names Docker

#### Scenario: The gate does not build the image

- **WHEN** a contributor runs `just --show check`
- **THEN** the output contains the line `check: check-server check-web`
