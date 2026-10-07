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

`just check-web` SHALL install the web shell's dependencies from `web/pnpm-lock.yaml`, then run four checks on `web/`:

- a type check of every TypeScript file in `web/`, including config files and test files;
- a lint that includes rules which read TypeScript types, and rules for React components and hooks;
- a format check, which fails when any file differs from the formatter's output;
- the unit tests.

`web/package.json` SHALL give each check its own script: `typecheck`, `lint`, `format:check`, and `test`. A contributor can then run one check alone.

`just check-web` SHALL exit with a non-zero status when any check fails, or when the lockfile does not match `web/package.json`. It SHALL NOT change any file that git tracks.

#### Scenario: The web shell is clean

- **WHEN** a contributor runs `just check-web` on an unchanged checkout inside the development shell
- **THEN** the command exits with status 0
- **AND** `git status --porcelain` prints nothing afterwards

#### Scenario: Dependencies are not installed yet

- **WHEN** `web/node_modules` does not exist and a contributor runs `just check-web`
- **THEN** the command installs the dependencies, runs the four checks, and exits with status 0

#### Scenario: A type error in app code

- **WHEN** a tester assigns a number to a `string` variable in a file under `web/src/`, then runs `just check-web`
- **THEN** the command exits with a non-zero status

#### Scenario: A type error in a config file

- **WHEN** a tester assigns a number to a `string` variable in `web/vite.config.ts`, then runs `just check-web`
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

`just dev` SHALL start the server and the web shell together. One interrupt, such as Ctrl-C, SHALL stop both. When either one exits, `just dev` SHALL stop the other and exit. It SHALL exit with a non-zero status when either one failed. `just dev` SHALL NOT clear the terminal, so earlier output, such as a cargo error, stays visible.

#### Scenario: A fresh clone runs the stack

- **WHEN** a contributor clones the repo and runs `just dev` inside the development shell
- **AND** opens `http://localhost:5173` once the server has built
- **THEN** the page shows `healthy`
- **AND** the repo root still has no `node_modules` folder

#### Scenario: Ctrl-C stops the whole stack

- **WHEN** `just dev` is running and the contributor presses Ctrl-C
- **THEN** `just` exits
- **AND** within 5 seconds nothing listens on port 5000 or port 5173

#### Scenario: The terminal keeps earlier output

- **WHEN** a tester runs `script -qc 'just dev' dev.log` inside the development shell on Linux
- **AND** stops it with Ctrl-C after the web shell's dev server listens on port 5173
- **THEN** `dev.log` contains none of these escape sequences: `ESC [ 2 J`, `ESC [ 0 J`, `ESC [ J`, or `ESC c`

#### Scenario: The web shell fails to start

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
