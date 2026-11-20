<!-- Plain language: Lead with the problem and resulting behavior. Give a reader enough context to decide whether the scope solves that problem. Keep the required headings. -->

## Why

Epic #496 moves Sidereal's commands from `just` to mise, so contributors install one tool runner, not two. The gate moved in #499. The fourteen recipes left in the `justfile` are the last reason to keep `just`, so this change moves them and deletes the `justfile`.

## What Changes

- **New mise tasks.** `mise.toml` defines one task for each recipe left in the `justfile`, with the recipe's name: `db-up`, `db-down`, `db-clean`, `db-url`, `db-test-url`, `db-test-clean`, `db-tls`, `db-schema`, `server`, `web`, `dev`, `e2e`, `instance-verify` and `external-db-verify` (D1, D2). Each one does what its recipe does today. The `default` recipe gets no task, because `mise tasks` already lists them.
- **Three longer bodies become POSIX `sh` scripts:** `dev`, `e2e` and `db-schema` (D3). The `dev` script runs the server and the web shell together, and stops both when either one exits, as `just dev` does today.
- **BREAKING: the `justfile` is deleted, and `just` leaves both routes.** `just <recipe>` stops working everywhere. `just` leaves `mise.toml`, `mise.lock`, the Nix shell (`nix/toolchains.nix`), and the version prints in the `mise` and `nix` workflows (D7).
- **CI runs the tasks.** The `e2e` job runs `mise run e2e`. The fixture steps in the `server` and `e2e` jobs run `mise run db-up`, `db-url`, `db-test-url` and `db-down` (D6).
- **Every mention of a `just` command names its `mise run` form** (D5): messages, comments and docs. That includes the test-support error messages in `server/crates/core/src/database/test_support.rs`, the usage lines in `scripts/*.sh`, `web/playwright.config.ts`, `web/vite.config.ts`, `compose.yml`, the three READMEs, `CONTRIBUTING.md`, `AGENTS.md`, `openspec/discovery.md` and the `visual-options` skill.
- **Contributors with neither mise nor Nix run the scripts or commands behind the tasks.** `CONTRIBUTING.md`'s hand-install steps drop `just`, and list what each task runs, as they already do for the four gate commands (D4).
- **The Nix shell cannot run the tasks,** because it has no mise. The docs send Nix users to mise on the host, or to the scripts behind the tasks, until #502 removes the Nix route (D8).
- **No spec names a `just` command once this merges** (D9, D10, D11). The stale requirements about `just skills` and `just enter` go. Commit 74da6ba removed both recipes when Let's Spec took over the skills, but the spec kept them.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `dev-commands`: every requirement that names a `just` recipe names its `mise run` task instead: the web shell, `dev`, the fixture URLs, and the eight `instance-verify` requirements. A new requirement lists the development tasks that `mise.toml` defines. The requirement "The justfile has no v0.10.x recipes" becomes one requirement: the repo has no root `justfile`, `mise.toml` names no `just` tool, and no mise task runs or checks the v0.10.x app (D9). The scenarios in "mise tasks hold the gate" that run `just --summary` and `just check` go. The Purpose names `mise.toml` as the place that runs everything.
- `ci`: the `e2e` job runs `mise run e2e`, and its order scenario names that line. The `server` job's path list drops the root `justfile`. The tool-command scenario's search pattern drops `just` (D11).
- `dev-environment`: `just` leaves the tool lists of the Nix shell and the mise route, and the `mise` and `nix` workflows' version checks (D7). The mise-lock scenarios that use `just` as their example tool use another tool. The gate scenario prepares the fixture with `mise run db-test-url`. Nine requirements about `just skills` and `just enter` are removed, and "Every supported agent sees the same skills" no longer runs `just skills` (D10).
- `compose-instance`: three retention and refusal requirements name `mise run instance-verify` instead of `just instance-verify` (D11).
- `web-shell`: the port-conflict scenario runs `mise run web` (D11).

## Impact

- **Code and config:** `mise.toml` (new tasks; `just` removed), `mise.lock`, the `justfile` (deleted), new scripts for `dev`, `e2e` and `db-schema`, `nix/toolchains.nix`, `.github/workflows/ci.yml` (`server` and `e2e` jobs), `.github/workflows/mise.yml` and `.github/workflows/nix.yml` (version prints), and the messages in `server/crates/core/src/database/test_support.rs`, `scripts/instance-verify.sh` and `scripts/external-db-verify.sh`.
- **Docs and comments:** `AGENTS.md`, `CONTRIBUTING.md`, `README.md`, `server/README.md`, `web/README.md`, `openspec/discovery.md`, `.agents/skills/visual-options/SKILL.md`, `web/playwright.config.ts`, `web/vite.config.ts` and `compose.yml`.
- **Contributors:** run `mise run <task>` instead of `just <recipe>`, with the same names. Nix-shell users run the tasks from a shell with mise, or run the scripts by hand.
- **Dependencies:** `just` is removed. Nothing is added.
- **Unchanged:** what the fixture, the Compose instance, the app image and the end-to-end suite do (D12). Only the command that runs them changes.
