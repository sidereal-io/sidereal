<!-- Plain language: Help the next implementer find the approach, understand the choices and act on the plan. Explain necessary technical terms. Put each risk beside its mitigation and each open question beside the decision it affects. -->

## Context

- The `justfile` holds fourteen recipes plus `default`. Eleven are one line that runs an existing script: `server/scripts/db-fixture.sh <verb>`, `server/scripts/db-tls.sh`, `scripts/instance-verify.sh` or `scripts/external-db-verify.sh`. `server` and `web` are one command each. `dev` uses `just`'s `[parallel]` attribute to run `server` and `web` together. `e2e` and `db-schema` are inline shell scripts, and both call `just db-url`, `just db-up` or `just db-test-url`.
- `mise.toml` already holds the five gate tasks, and a comment that states the task-body rule: up to about six lines inline, and a longer body in a POSIX `sh` script under `scripts/` or `server/scripts/`.
- mise runs a task from the directory of the `mise.toml` that defines it, which is the repo root here. It puts the pinned tools on `PATH` for the task. With one task, `mise run` adds no output prefix, and its standard output is only what the task prints. It also writes a `[<task>] $ <command>` line to standard error, unless the task sets `quiet = true`. All three behaviors were checked with mise 2026.10.17.
- mise's own parallel runs (`depends`, or `mise run a ::: b`) keep the other task running when one task exits with status 0. A trial confirmed this, so mise cannot give `dev` its stop-both behavior.
- `dash`, which is `/bin/sh` on Debian and Ubuntu, turns job control off when no terminal is attached: `set -m` prints `can't access tty` and background jobs stay in the parent's process group. A trial confirmed this. In a shell without job control, background jobs also ignore SIGINT, so Ctrl-C does not reach them.
- `cargo run` replaces its own process with the program it runs, so the server's process ID is the one the shell started. `pnpm dev` starts Vite as a child process.
- The `ci` workflow's `server` and `e2e` jobs prepare the fixture with `mise exec -- just db-up` and read a URL with `mise exec -- just db-test-url` or `db-url`. The `e2e` job's test step is `mise exec -- just e2e`.
- `just` is installed in three places: `mise.toml` (with its `mise.lock` entry), `nix/toolchains.nix` (`pkgs.just`), and the hand-install steps in `CONTRIBUTING.md`. The `mise` and `nix` workflows print `just --version`.

## Goals / Non-Goals

**Goals:**

- Each task does what its recipe did, so contributors type `mise run <name>` where they typed `just <name>`.
- Every task body runs without mise: a contributor with neither mise nor Nix can copy each command from `CONTRIBUTING.md`, and no script calls `mise` or `just`.
- One definition of each command. The server command and the web command each live in one place.

**Non-Goals:**

- Adding mise to the Nix shell, or removing the Nix route. That is #502.
- Changing what the fixture, the Compose instance, the app image or the end-to-end suite do.
- Renaming the word "recipe" in requirements that name no `just` command, such as the `postgres-foundation` and `external-postgres` requirements about "root recipes" and "the deployment recipe". They describe behavior that still holds, and #502 rewrites the remaining Nix-era wording.
- Documenting `openssl` as a system prerequisite. Epic #496 gives that to the docs story.

## Test Seams

- **Primary test seam:** the `mise run <task>` command line at the repo root, read through its exit status, its output and the processes and ports it leaves behind. `mise tasks ls --json` and `mise tasks info <task> --json`, read with `jq`, show the tasks and their bodies without running them.
- **Existing seams reused:** `yq` over `.github/workflows/ci.yml`, as the `ci` spec's scenarios already do; `openspec validate --strict`; and the `ci`, `mise` and `nix` workflow runs on this pull request. The `nix` workflow runs because this change edits `nix/toolchains.nix`.

## Scrutiny Tier & Blast Radius

- **Tier:** medium
- **Blast radius:** contributor tooling across the repo: `mise.toml`, `mise.lock`, the deleted `justfile`, three new scripts, `nix/toolchains.nix`, three workflows, two error messages in `server/crates/core/src/database/test_support.rs`, comments in `web/` and `compose.yml`, and the docs. No product behavior, schema, API or trust boundary changes. The new `dev` script supervises two processes and handles signals, which is the one piece of new logic. A mistake shows up as a broken `mise run dev`, a red CI job, or a leftover process on port 5000 or 5173.

## Decisions

### D-1. Each task runs one command, and three bodies become scripts

| Task | Body | Settings |
|---|---|---|
| `db-up` | `sh server/scripts/db-fixture.sh up` | |
| `db-down` | `sh server/scripts/db-fixture.sh down` | |
| `db-clean` | `sh server/scripts/db-fixture.sh clean` | |
| `db-url` | `sh server/scripts/db-fixture.sh url` | `quiet = true` |
| `db-test-url` | `sh server/scripts/db-fixture.sh test-url` | `quiet = true` |
| `db-test-clean` | `sh server/scripts/db-fixture.sh orphan-clean` | |
| `db-tls` | `sh server/scripts/db-tls.sh` | |
| `db-schema` | `sh server/scripts/db-schema.sh` (new) | |
| `server` | `sh scripts/dev.sh server` (new) | `raw = true` |
| `web` | `sh scripts/dev.sh web` | `raw = true` |
| `dev` | `sh scripts/dev.sh` | `raw = true` |
| `e2e` | `sh scripts/e2e.sh` (new) | |
| `instance-verify` | `sh scripts/instance-verify.sh` | |
| `external-db-verify` | `sh scripts/external-db-verify.sh` | |

- Each task gets a `description`, taken from the recipe's comment, which `mise tasks ls` prints.
- **`quiet = true` on the two URL tasks** stops mise from writing its `[<task>] $ <command>` line. Then the URL is the only output on either stream, which the "Production configuration stays private" scenario reads. Command substitution, as in `export DATABASE_URL=$(mise run db-url)`, already gets only the URL, because mise writes that line to standard error.
- **`raw = true` on the three long-running tasks** connects the task straight to the terminal. Vite and the server then see a terminal, and mise adds nothing to their output that could clear the screen.
- **Why `server` and `web` run `scripts/dev.sh`:** `dev` cannot call `mise run server` (the scripts must run without mise), so the two commands need one home that all three tasks share. Rejected: an inline body for `server` and `web`, with copies of the same commands in `scripts/dev.sh`. The copies would drift.
- `e2e` and `db-schema` keep their recipe bodies, with each `just db-*` call replaced by the script it ran: `sh server/scripts/db-fixture.sh url`, `up` or `test-url`.

### D-2. `scripts/dev.sh` supervises the two processes itself

`scripts/dev.sh server` replaces itself with `cargo run -p sidereal-server --manifest-path server/Cargo.toml`. `scripts/dev.sh web` changes to `web/`, runs `pnpm install --frozen-lockfile --reporter=append-only`, then replaces itself with `pnpm dev`. With no argument, the script:

1. Starts `sh scripts/dev.sh server` and `sh scripts/dev.sh web` in the background, and records both process IDs.
2. Traps INT, TERM and HUP. The trap stops both process trees and exits with status 130 for INT, or 143 for TERM and HUP.
3. Checks every second, with `kill -0`, whether both are still running. When one has exited, it reads that one's status with `wait`.
4. Stops the other process tree, waits for it, and exits with the first one's status. A non-zero status therefore means that the one that exited first failed.

To stop a process tree, the script lists every process with `ps -A -o pid= -o ppid=`, finds the descendants of the recorded ID, and sends them and the recorded process SIGTERM. Both Linux and macOS support those `ps` options.

- **Why not job-control process groups (`set -m` and `kill -- -<pgid>`):** `dash` turns job control off without a terminal, and CI and agents can run the script with no terminal attached. Walking the process tree works with or without one.
- **Why polling instead of a named pipe:** a one-second poll is simple to get right with traps, and the specs allow 5 seconds to free the ports and 10 seconds to exit after a failed start.
- **Rejected: mise's own parallel run.** It keeps the other task alive when one exits with status 0, and it needs mise, which a contributor on the hand-install route does not have.

### D-3. `just` leaves every route

- Remove `just = "1"` from `mise.toml`, run `mise lock`, and check that the `[[tools.just]]` entry and its platform tables are gone from `mise.lock`. If `mise lock` leaves them, delete them by hand. Then `mise install --locked` must still succeed, which the `ci` jobs exercise.
- Remove `pkgs.just` from `nix/toolchains.nix`, and fix its header comment. `flake.lock` does not change, because `just` came from the locked nixpkgs.
- Drop `just --version` from the version prints in `.github/workflows/mise.yml` and `.github/workflows/nix.yml`.
- Delete the `justfile`.

### D-4. CI calls the tasks

- The `server` job's fixture step runs `mise run db-up` and `test_url=$(mise run db-test-url)`. Its stop step runs `mise run db-down`.
- The `e2e` job's fixture step runs `mise run db-up` and `db_url=$(mise run db-url)`. Its test step runs `mise run e2e`, and its stop step runs `mise run db-down`. The comment above the Chromium step names the `e2e` task.
- The `ci` spec's search for tool commands outside mise drops `just` from its pattern, because no step runs `just` and it is no longer installed.

### D-5. Docs and messages name `mise run`

Every `just <recipe>` in the files the proposal lists becomes `mise run <recipe>`. These need more than a word swap:

- **`CONTRIBUTING.md`:** the Development Commands block lists every task. The hand-install steps drop `just` from step 4 and gain a table with one row per development task: the task and the command it runs, copied from `mise.toml`. The Nix section says that the Nix shell has no mise: Nix users run the tasks with mise on the host, or run the commands from that table. "Where versions are set" loses its `just` row, and the sentence about tools that both locks pin drops `just`.
- **`AGENTS.md`:** the Stack line says the tasks in `mise.toml` run everything, instead of "orchestrated by the root `justfile`".
- **`server/crates/core/src/database/test_support.rs`:** the two panic messages say `run mise run db-up, then export TEST_DATABASE_URL=$(mise run db-test-url)`. They are test-support messages only, so no test asserts on them.
- **`web/playwright.config.ts`:** the comment on the server's `webServer` block says it runs the same command as `mise run server`.

### D-6. The build edits the `dev-commands` Purpose directly

The Purpose still says the root `justfile` runs the server, the web shell and the fixture. A delta cannot change a Purpose, so the build edits `openspec/specs/dev-commands/spec.md` in the docs commit: the tasks in `mise.toml` run the server, the web shell, the database fixture and the gate. A trial archive of this change's deltas, in a scratch copy, left that Purpose as the only `justfile` mention in any spec.

No decision here needs its own record. mise was chosen as the tool manager in ADR-016, and epic #496's shared decisions set the task-body rule.

## Risks / Trade-offs

- [`scripts/dev.sh` leaves a process behind, such as Vite's Node child, after Ctrl-C or a failed start] → The tree walk in D-2 stops descendants, not only the recorded process. The "Ctrl-C stops the whole stack" and "The web shell fails to start" scenarios check ports 5000 and 5173 and the process list. Run them under both `dash` and `bash --posix`.
- [A trap does not run while the script waits] → The script waits in `sleep 1`, a foreground command that a signal interrupts, not in a blocking read. Running the Ctrl-C scenario under `dash` tests this.
- [`raw = true` changes how mise handles Ctrl-C] → The scenarios run through `mise run dev`, not only through the script. If `raw` stops mise from passing the signal on, drop `raw` and run the "terminal keeps earlier output" scenario again to confirm that mise adds no screen-clearing sequence.
- [The commands in `CONTRIBUTING.md` drift from `mise.toml`] → The "listed commands match the tasks" scenario compares them, and the docs task verifies it.
- [A contributor in the Nix shell types `mise run dev` and gets "command not found", or types `just dev` from habit] → The docs say what to run instead (D-5). The gap lasts until #502 removes the Nix route.
- [The `ci` pattern no longer catches a stray `just` call] → `just` is not installed on any route, so a stray call fails its job at once.
- [An open branch still calls `just` in CI] → It fails on its next push. Its author renames the call to `mise run`, with the same task name.

## Migration & Rollback

- **Migration:** contributors run `mise run <name>` instead of `just <name>`; every name is the same. Nobody needs to reinstall anything; `mise prune` removes the unused `just` install. No data or schema changes.
- **Rollback:** revert this pull request. The `justfile`, the `just` pins, the CI steps and the docs come back together, because they change in the same pull request.
