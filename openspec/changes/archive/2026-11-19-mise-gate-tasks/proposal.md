## Why

The gate is the command every contributor and agent runs before a pull request, and CI runs its four parts as separate jobs. Today it is `just check` in the `justfile`. Epic #496 moves every recipe to mise tasks so `just` can go. The gate moves first, on its own, so that *Work & check* and *Open PR* keep working at each step of the move.

## What Changes

- **New mise tasks.** `mise.toml` defines `check-compose`, `check-openspec`, `check-server`, `check-web` and `check`. Each part does what its `justfile` recipe does today (D1). Each task body stays inline, because each fits in about six lines (D2).
- **`mise run check` runs the parts in order and stops at the first failure:** Compose, OpenSpec, server, then web (D4).
- **BREAKING: the five gate recipes leave the `justfile`.** `just check` and `just check-*` stop working. The other recipes stay until #500, so `just` stays installed (D3).
- **CI runs the tasks.** The `compose`, `openspec`, `server` and `web` jobs run `mise run <task>` instead of `mise exec -- just <recipe>` (D5). The `e2e` and `lets-spec` jobs and the fixture steps do not change.
- **Every mention of a gate command names its `mise run` form** (D6). That covers `AGENTS.md`, `CONTRIBUTING.md`, the three READMEs, `openspec/config.yaml`, `openspec/discovery.md`, `docs/security-scanning.md`, the usage message in `scripts/external-db-verify.sh`, and the spec scenarios listed below. `just external-db-verify` stays until #500.
- **The Nix route runs the gate through mise** until #502 removes that route (D7). The Nix shell has no mise, so `nix develop --command just check` stops being a supported command.
- **Contributors with neither mise nor Nix run the four parts' commands by hand.** `CONTRIBUTING.md` lists them in order, and they are the commands the tasks run (D8).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `dev-commands`: the web and OpenSpec gate requirements, and the requirement that keeps `instance-verify` out of the gate, name the mise tasks instead of `just` recipes. The full-gate order is read from `mise.toml` instead of `just --show check`.
- `ci`: the four check jobs run `mise run <task>`. A job's checks are one mise task instead of one `just` recipe. A tool command may run as `mise run <task>` as well as `mise exec -- <command>`. The web and OpenSpec job requirements name the tasks.
- `dev-environment`: the gate requirement names `mise run check` and its four parts instead of `nix develop --command just check` (D7). The scenario for a contributor without Nix runs the four parts' commands instead of `just check` (D8).
- `design-tokens`: scenarios that run `just check-web` run `mise run check-web`.
- `external-postgres`: the daemon-free Compose scenario runs `mise run check-compose`.

## Impact

- **Code and config:** `mise.toml` (new `[tasks.*]` tables), `justfile` (five recipes removed), `.github/workflows/ci.yml` (four check steps), and the usage message in `scripts/external-db-verify.sh`.
- **Docs:** `AGENTS.md`, `CONTRIBUTING.md`, `README.md`, `server/README.md`, `web/README.md`, `openspec/config.yaml` context, `openspec/discovery.md` and `docs/security-scanning.md`.
- **Contributors:** run `mise run check` instead of `just check`. Anyone working in the Nix shell, which has no mise, runs the gate from a shell with mise, or runs the four commands by hand.
- **Dependencies:** none new. mise and its lock already provide every tool the gate runs. `just` stays in `mise.toml` and in the hand-install steps until #500.
- **Unchanged:** the e2e job, the fixture recipes, `instance-verify`, `external-db-verify`, and the gate's checks themselves.
