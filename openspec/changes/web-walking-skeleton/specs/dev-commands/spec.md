## Purpose

The root `justfile` is the one place to run each stack for development. Its recipes start the v2 server, the v2 web shell, and the v0.10.x stack, each by name.

## ADDED Requirements

### Requirement: One command runs the v2 stack

`just dev` SHALL start the v2 server and the v2 web shell together. It SHALL NOT need any package from the root npm workspace. One interrupt, such as Ctrl-C, SHALL stop both. When either one exits, `just dev` SHALL stop the other and exit. It SHALL exit with a non-zero status when either one failed. `just dev` SHALL NOT clear the terminal, so earlier output, such as a cargo error, stays visible.

#### Scenario: A fresh clone runs the v2 stack

- **WHEN** a contributor clones the repo and runs `just dev` inside the development shell, without running `npm install` first
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

### Requirement: The web shell runs on its own

`just web` SHALL install the web shell's dependencies from `web/pnpm-lock.yaml`, then start the web shell's dev server. It SHALL NOT change `web/pnpm-lock.yaml`. It SHALL stop with an error when the lockfile does not match `web/package.json`.

#### Scenario: Dependencies are not installed yet

- **WHEN** `web/node_modules` does not exist and a contributor runs `just web`
- **THEN** the web shell's dev server starts on port 5173

#### Scenario: The lockfile is out of date

- **WHEN** a contributor adds a dependency to `web/package.json` without updating `web/pnpm-lock.yaml`, then runs `just web`
- **THEN** the command exits with a non-zero status before the dev server starts
- **AND** `git diff --exit-code web/pnpm-lock.yaml` exits with status 0

### Requirement: The v0.10.x stack runs by explicit name

`just v0-dev` SHALL run the whole v0.10.x stack: its server, its worker, and its frontend. `just v0-frontend` SHALL run the v0.10.x frontend only. The `justfile` SHALL NOT define a recipe named `frontend`.

#### Scenario: A contributor runs the v0.10.x stack

- **WHEN** a contributor with the v0.10.x dependencies installed runs `just v0-dev`
- **THEN** the v0.10.x server listens on port 5000
- **AND** the v0.10.x frontend listens on port 5173

#### Scenario: The old recipe name is gone

- **WHEN** a contributor runs `just frontend`
- **THEN** the command exits with a non-zero status

### Requirement: Recipes are grouped by stack

Every recipe that runs or checks one stack SHALL belong to that stack's group: `v2` or `v0.10.x`. Recipes that serve the whole repo, such as `skills`, SHALL belong to no stack group.

#### Scenario: A reviewer reads the recipe groups

- **WHEN** a reviewer runs `just --dump --dump-format json` and reads each recipe's `group` attribute
- **THEN** `dev`, `server`, `web`, `check`, and `check-server` are in group `v2`
- **AND** `v0-dev` and `v0-frontend` are in group `v0.10.x`
- **AND** `default`, `skills`, and `enter` have no group
