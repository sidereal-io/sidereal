## MODIFIED Requirements

### Requirement: Recipes are grouped by stack

Every recipe that runs or checks one stack SHALL belong to that stack's group: `v2` or `v0.10.x`. Recipes that serve the whole repo, such as `skills`, SHALL belong to no stack group.

#### Scenario: A reviewer reads the recipe groups

- **WHEN** a reviewer runs `just --dump --dump-format json` and reads each recipe's `group` attribute
- **THEN** `dev`, `server`, `web`, `check`, `check-server`, and `check-web` are in group `v2`
- **AND** `v0-dev` and `v0-frontend` are in group `v0.10.x`
- **AND** `default`, `skills`, and `enter` have no group

## ADDED Requirements

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
