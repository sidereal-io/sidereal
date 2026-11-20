<!-- Plain language: Each task must make sense to an agent and reviewer without the planning conversation. Name an observable outcome, its location when needed, and how to verify it. Preserve checkbox syntax and tags. -->

Run every command from the repo root. `design.md` D-2 gives each task body exactly. The server checks need the PostgreSQL fixture: run `mise exec -- just db-up` once, and set `TEST_DATABASE_URL` to the output of `mise exec -- just db-test-url`.

## 1. The gate as mise tasks

- [x] 1.1 [easy] `mise run check-compose` runs `sh scripts/compose-config-check.sh` and passes on an unchanged checkout. Add the task, with a `description`, to `mise.toml` (verify: `mise run check-compose`) (✓ `mise run check-compose` passed)
- [x] 1.2 [easy] `mise run check-openspec` runs `OPENSPEC_TELEMETRY=0 openspec validate --all --strict --no-interactive`, passes without waiting for input, and leaves the tree unchanged (verify: `mise run check-openspec < /dev/null && test -z "$(git status --porcelain -- openspec)"`) (✓ `mise run check-openspec < /dev/null` passed, 12 items; `git status --porcelain -- openspec` empty)
- [ ] 1.3 [easy] `mise run check-server` runs the format check, clippy with warnings denied, and `cargo test` in `server/`, then `server/scripts/check-arch.sh` from the repo root, with `cd` scoped inside a subshell (verify: `TEST_DATABASE_URL=$(mise exec -- just db-test-url) mise run check-server`)
- [ ] 1.4 [easy] `mise run check-web` installs from `web/pnpm-lock.yaml` and runs the six web checks, leaving every tracked file unchanged (verify: `mise run check-web && git diff --exit-code -- web`)
- [ ] 1.5 [medium] `mise run check` runs the four parts as a `run` list of task references, in the order `check-compose`, `check-openspec`, `check-server`, `check-web`, with no `depends`, so it stops at the first failure (verify: `test "$(mise tasks info check --json | jq -r '.run[].task' | paste -sd' ')" = 'check-compose check-openspec check-server check-web' && test "$(mise tasks info check --json | jq '.depends | length')" = 0`)
- [ ] 1.6 [easy] The `justfile` no longer defines `check`, `check-compose`, `check-openspec`, `check-server` or `check-web`, and its other recipes still list (verify: `! just --summary | tr ' ' '\n' | grep -qE '^check(-compose|-openspec|-server|-web)?$' && just --summary | tr ' ' '\n' | grep -qx db-up`)

## 2. CI runs the tasks [depends-on: 1]

- [ ] 2.1 [parallel] [easy] The `compose`, `server`, `web` and `openspec` jobs in `.github/workflows/ci.yml` each run their check as `mise run check-<part>`; the fixture steps and the `e2e` and `lets-spec` jobs do not change (verify: `for j in compose web openspec; do test "$(yq ".jobs.$j.steps[] | select(has(\"run\")) | .run" .github/workflows/ci.yml)" = "mise run check-$j" || exit 1; done && test "$(yq '.jobs.server.steps[] | select(has("run")) | .run' .github/workflows/ci.yml | grep 'check-')" = 'mise run check-server'`)

## 3. Docs name the mise tasks [depends-on: 1]

- [ ] 3.1 [parallel] [easy] `AGENTS.md`, `README.md`, `server/README.md`, `web/README.md` and `.agents/skills/visual-options/SKILL.md` name `mise run check` and `mise run check-<part>` wherever they named a gate recipe. Edit `AGENTS.md` outside the lets-spec block only (verify: `! grep -nE 'just check' AGENTS.md README.md server/README.md web/README.md .agents/skills/visual-options/SKILL.md`)
- [ ] 3.2 [parallel] [medium] `CONTRIBUTING.md` names the mise tasks in its steps, Development Commands, Testing and Pull Request sections; its hand-install route lists the four gate commands from `design.md` D-2 in gate order; its Nix section documents the transitional gate options (the four commands in the shell or `mise run check` on the host); and it states the task-body rule: up to about six lines inline in `mise.toml`, longer bodies in a POSIX `sh` script under `scripts/` or `server/scripts/` (verify: `! grep -n 'just check' CONTRIBUTING.md && grep -qF 'OPENSPEC_TELEMETRY=0 openspec validate --all --strict --no-interactive' CONTRIBUTING.md && grep -qF '(cd web && pnpm install --frozen-lockfile --reporter=append-only && pnpm typecheck' CONTRIBUTING.md`)
- [ ] 3.3 [parallel] [easy] The context in `openspec/config.yaml`, *Work & check* in `openspec/discovery.md`, `docs/security-scanning.md`, and the usage message in `scripts/external-db-verify.sh` name `mise run check` or `mise run check-compose`; the usage message keeps `just external-db-verify`. The Purpose of `openspec/specs/dev-commands/spec.md` says the `justfile` runs the server, web shell and fixtures, and `mise.toml`'s tasks run the gate (`design.md` D-5) (verify: `! grep -n 'just check' openspec/config.yaml openspec/discovery.md docs/security-scanning.md scripts/external-db-verify.sh && sed -n '/^## Purpose/,/^## Requirements/p' openspec/specs/dev-commands/spec.md | grep -q 'mise.toml'`)

## 4. The whole gate passes [depends-on: 2, 3]

- [ ] 4.1 [medium] With the fixture up, `mise run check` exits 0, and no file outside `openspec/changes/` still names a removed gate recipe as a command (verify: `TEST_DATABASE_URL=$(mise exec -- just db-test-url) mise run check && ! git grep -nE 'just check' -- ':!openspec/changes' ':!openspec/specs'`)
- [ ] 4.2 [easy] Every job of the `ci` workflow passes on pull request #512 after the branch is pushed (verify: `gh pr checks 512`)

## Deviations Ledger

<!-- Dated notes on small departures from the plan, written during apply. Stop and ask the user about a large one. -->
