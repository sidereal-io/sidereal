<!-- Plain language: Write questions and answers so a reader without the conversation can understand each decision and its source. Keep the confirmation marker exactly as written. -->

## Story

#500 mise-tasks — every justfile recipe is a mise task, and the justfile is gone. https://github.com/sidereal-io/sidereal/issues/500

## Decisions

| # | Question | Answer | Source |
|---|---|---|---|
| D1 | Which recipes become mise tasks? | Every recipe left in the `justfile`: `db-up`, `db-down`, `db-clean`, `db-url`, `db-test-url`, `db-test-clean`, `db-tls`, `db-schema`, `server`, `web`, `dev`, `e2e`, `instance-verify` and `external-db-verify`. Each does what its recipe does today. The `default` recipe (`just --list`) gets no task, because `mise tasks ls` already lists them. | Evidence: `justfile`, #500 body (Scope) |
| D2 | Do the tasks keep the recipe names, or use mise's namespaced style (`db:up`)? | Keep the recipe names, as the `check-*` gate tasks do: `mise run db-up`, `mise run dev`, and so on. | Assumption |
| D3 | Where does each task body live? | Up to about six lines inline in `mise.toml`; a longer body goes in a POSIX `sh` script under `scripts/` or `server/scripts/`. `e2e`, `db-schema` and `dev` become scripts. `dev` runs `server` and `web` together and stops both when either exits. | Evidence: #500 body (Scope), `mise.toml` (task-body comment); Assumption (`dev` as a script) |
| D4 | How does a contributor with neither mise nor Nix run `dev`, the fixture, `e2e` and `db-schema` once `just` is gone? | `CONTRIBUTING.md`'s hand-install steps drop `just` and list the plain command or `sh` script behind each task, as they already do for the four gate commands. | Assumption |
| D5 | Which text that says `just <recipe>` changes? | All of it, to the new `mise run` command: messages, comments and docs, including `server/crates/core/src/database/test_support.rs`, `scripts/*.sh`, `web/playwright.config.ts`, `web/vite.config.ts`, `compose.yml`, `README.md`, `CONTRIBUTING.md`, `AGENTS.md`, `server/README.md`, `web/README.md`, `openspec/discovery.md` and `.agents/skills/visual-options/SKILL.md`. | Evidence: #500 body (Scope), `git grep just` |
| D6 | Which CI steps change? | The `e2e` job runs the mise tasks for the fixture and the tests. The `server` job's fixture steps also switch, because they call `just db-up`, `just db-test-url` and `just db-down`. The `mise` workflow stops printing `just --version`. | Evidence: #500 body (Scope), `.github/workflows/ci.yml`, `.github/workflows/mise.yml` |
| D7 | Does `just` leave the Nix shell in this change, or wait for #502? | Leave it in this change: `nix/toolchains.nix`, the `nix` workflow's version print and the dev-environment toolchain requirement drop `just`. Nothing runs `just` any more. | Assumption |
| D8 | The Nix shell has no mise, so inside it nobody can run the new tasks. Accept that? | Accept the gap until #502 removes the Nix route, as the gate story did. The docs point Nix users to mise on the host, or to the scripts behind the tasks. | Assumption |
| D9 | What replaces the `dev-commands` scenarios that prove the v0.10.x recipes and the gate recipes are gone (`just --summary`, `just check`, `just v0-dev`)? | One requirement: the repo has no root `justfile`, `mise.toml` has no `just` tool, and no mise task runs or checks the v0.10.x app. | Assumption |
| D10 | About six dev-environment requirements still describe `just skills` and `just enter`, which commit 74da6ba removed when Let's Spec took over the skills. Remove them now, or leave them for #502? | Remove them in this change, so no spec names a `just` command once it merges. | Assumption |
| D11 | Do the other specs that name a `just` command change too? | Yes. Every scenario and requirement in `ci`, `compose-instance`, `web-shell`, `dev-environment` and `dev-commands` that names a `just` command names its `mise run` task instead. The `ci` requirement for the `server` job drops "the root `justfile`" from its file list. | Evidence: `git grep just -- openspec/specs` |
| D12 | What stays out of scope? | What the fixture, the Compose instance and the app image do. Only the command that runs them changes. | Evidence: #500 body (Scope, out) |

## Changes from the story

- Added: the `server` CI job's fixture steps and the `mise` workflow's version print change (D6). The story named only the `e2e` job, but both other places call `just`, which this change removes.
- Added: `just` leaves the Nix shell and the `nix` workflow now, not in #502 (D7).
- Added: the stale `just skills` and `just enter` requirements leave the dev-environment spec (D10).
- Added: the `ci`, `compose-instance`, `web-shell` and `dev-environment` specs change as well as `dev-commands` (D11), because each names a `just` command that will no longer exist.
- Added: `CONTRIBUTING.md` lists the command or script behind each task for contributors with neither mise nor Nix (D4).

lets-go confirmed this ledger under the user's standing instruction to stop in the brainstorm only when a question has no recommendation; the user did not read it in person.
Confirmed by user: yes
