## Context

- The gate is five `justfile` recipes: `check` and its four parts. `check` lists the parts as `just` dependencies, which `just` runs one at a time, in order, and stops at the first failure.
- `mise.toml` has `[tools]`, `[env]`, `[hooks]` and `[settings]` tables, and no tasks yet. mise puts its tools on `PATH` when it runs a task, so a task needs no `mise exec --` inside it.
- The `ci` workflow's four check jobs install from `mise.lock` with `jdx/mise-action`, which leaves `mise` on `PATH`. Each job runs `mise exec -- just check-<part>`.
- The Nix shell has no mise. The `nix` workflow builds the shell and prints versions, and runs no gate.
- mise starts a task's `depends` in parallel. A `run` list runs its entries one at a time and stops at the first one that fails. A `run` entry can name another task as `{ task = "<name>" }`. A trial with mise 2026.10.17 confirmed this: with three entries where the second fails, the third never started, and `mise run` exited with the failing entry's status.

## Goals / Non-Goals

**Goals:**

- One definition of the gate, in `mise.toml`, that contributors, agents and CI all run.
- Each task runs the same commands as the recipe it replaces, so the checks themselves do not change.
- A contributor with neither mise nor Nix can copy the same four commands from `CONTRIBUTING.md`.

**Non-Goals:**

- Moving `dev`, the fixture recipes, `e2e`, `db-schema`, `db-tls`, `instance-verify` or `external-db-verify`. That is #500.
- Adding mise to the Nix shell, or removing the Nix route. That is #502.
- Changing what any check checks, or the `e2e` and `lets-spec` jobs.
- The path list in the `ci` spec's "The server checks run when server code changes" requirement. The workflow has no path filter today, so this change leaves that list alone.

## Test Seams

- **Primary test seam:** the `mise run` command line at the repo root, read through its exit status and output. `mise tasks info check --json`, read with `jq`, shows the gate's order without running it. The specs' scenarios use both.
- **Existing seams reused:** `yq` over `.github/workflows/ci.yml`, as the `ci` spec's scenarios already do; `just --summary`; and the `ci` workflow run on this pull request, which shows each job's step passing.

## Scrutiny Tier & Blast Radius

- **Tier:** low
- **Blast radius:** contributor tooling only: `mise.toml`, the `justfile`, four steps of `.github/workflows/ci.yml`, one usage message, and docs. No product code, schema, API or trust boundary changes. A mistake shows up as a red or wrongly green gate, which CI on this pull request exercises.

## Decisions

### D-1. `check` is a `run` list of task references

```toml
[tasks.check]
description = "The gate to pass before every pull request"
run = [
  { task = "check-compose" },
  { task = "check-openspec" },
  { task = "check-server" },
  { task = "check-web" },
]
```

- **Why:** a `run` list runs one entry at a time and stops at the first failure, which is the order and stop the specs require (D4 in the ledger). `mise tasks info check --json` exposes the list, so a test can read the order without running the gate.
- **Rejected: `depends = [...]`.** mise starts dependencies in parallel, so the order and the stop would be lost. `--jobs 1` serializes them, but it is a flag the caller passes or a project-wide setting, not part of the task. As a setting it would also serialize #500's `dev` task, which runs the server and the web shell together.
- **Rejected: `run = "mise run check-compose && mise run check-openspec && ..."`.** It works, but nests a mise process per part, and hides the order inside a shell string that `jq` cannot read.

### D-2. Each part's body stays inline and keeps the recipe's commands

All four bodies fit the six-line rule from the ledger (D2), so no new script is needed. Each task gets a `description`, which `mise tasks` prints, in place of the recipe comment.

| Task | Body |
|---|---|
| `check-compose` | `run = "sh scripts/compose-config-check.sh"` |
| `check-openspec` | `run = "OPENSPEC_TELEMETRY=0 openspec validate --all --strict --no-interactive"` |
| `check-server` | `run = "(cd server && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test) && server/scripts/check-arch.sh"` |
| `check-web` | `run = "(cd web && pnpm install --frozen-lockfile --reporter=append-only && pnpm typecheck && pnpm lint && pnpm format:check && pnpm design:lint && pnpm tokens:check && pnpm test)"` |

- **Why the bodies use subshells `(...)` for `cd`:** wrapping directory changes in subshells ensures compound commands preserve the repository root working directory when a contributor runs them sequentially in an interactive POSIX shell. `CONTRIBUTING.md` lists these same four commands for the hand-install route (D8), so the text there matches `mise.toml` exactly.
- Every task runs from the repo root, which is mise's default for a task in the root `mise.toml`.

### D-3. CI calls `mise run <task>` directly

Each of the four check steps becomes `run: mise run check-<part>`. The server job's fixture steps keep `mise exec -- just db-up`, `db-test-url` and `db-down`, because those recipes stay until #500.

The `ci` spec's grep for tool commands outside `mise exec --` looks for `cargo`, `pnpm`, `just`, `lets` and `openspec` followed by a space. `mise run check-server` contains none of them, so the scenario still passes. The requirement text now names `mise run <task>` as the second allowed form.

### D-4. The hand-install route lists the four commands

`CONTRIBUTING.md`'s hand-install steps gain a numbered list of the four commands from D-2, in gate order, with one line each on what it checks. No `scripts/check.sh` is added (ledger D8), so the order lives only in `mise.toml` and in that list. Under `### With Nix`, it also documents how Nix contributors run checks (running the four gate commands in the shell, or `mise run check` on the host) until issue #502 removes the Nix route.

### D-5. Purpose of the `dev-commands` spec

That spec's Purpose says the `justfile` is "the one place" to run the checks. A delta cannot change a Purpose, so the build edits the main spec's Purpose directly, in the same commit as the docs: the `justfile` runs the server, the web shell and the fixtures, and `mise.toml`'s tasks run the gate.

No decision here needs its own record. ADR-016 already chose mise as the task runner, and epic #496's shared decisions set the task-body rule.

## Risks / Trade-offs

- [The four commands in `CONTRIBUTING.md` drift from `mise.toml`] → The verify step of the docs task compares each listed command with the task body, and the plan review and code review check both. A later change can add a test if drift happens.
- [`mise run` behaves differently in CI, where there is no terminal, for example in how it prefixes output] → The spec scenarios assert on exit status and on the absence of a part's prefix, not on exact output. The `ci` workflow on this pull request runs every task without a terminal.
- [A contributor in the Nix shell types `just check` and gets "recipe not found"] → `AGENTS.md`, `CONTRIBUTING.md` and the READMEs name `mise run check`. The gap lasts until #502 removes the Nix route (ledger D7).
- [mise-action installs a mise release too old for `{ task = ... }` entries] → mise-action installs the latest mise by default, and the `ci` run on this pull request would fail at the `check-*` steps if the form were unsupported.

## Migration & Rollback

- **Migration:** contributors run `mise run check` instead of `just check`. No data or schema changes.
- **Rollback:** revert this pull request. The `justfile` recipes, the CI steps and the docs come back together, because they change in the same pull request.
