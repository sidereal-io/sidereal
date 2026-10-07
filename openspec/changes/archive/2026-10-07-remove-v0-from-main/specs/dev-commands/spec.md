## REMOVED Requirements

### Requirement: One command runs the v2 stack

**Reason**: Replaced by "One command runs the server and the web shell". The root npm workspace leaves `main`, so the requirement no longer mentions it, and no scenario calls the stack "v2".

**Migration**: None. `just dev` behaves as before.

### Requirement: The v0.10.x stack runs by explicit name

**Reason**: The v0.10.x app leaves `main`. Its recipes `v0-dev` and `v0-frontend` have nothing to run.

**Migration**: Check out the `v0.x` branch and run `npm run dev` for the whole v0.10.x stack, or `npm run dev:frontend` for its frontend.

### Requirement: Recipes are grouped by stack

**Reason**: `main` holds one stack, so a group per stack adds nothing. `just --list` shows every recipe in one list.

**Migration**: None. Every recipe keeps its name.

## ADDED Requirements

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
