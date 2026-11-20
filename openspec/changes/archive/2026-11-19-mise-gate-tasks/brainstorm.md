<!-- Plain language: Write questions and answers so a reader without the conversation can understand each decision and its source. Keep the confirmation marker exactly as written. -->

## Story

#499 mise-gate-tasks — mise run check is the gate, on contributors' machines and in CI. https://github.com/sidereal-io/sidereal/issues/499

## Decisions

| # | Question | Answer | Source |
|---|---|---|---|
| D1 | Which recipes become mise tasks in this story? | `check-compose`, `check-openspec`, `check-server`, `check-web` and `check`, each doing what its `justfile` recipe does; they leave the `justfile`. The other recipes stay there for #500. | Evidence: #499 body (Scope) |
| D2 | Where does each task body live? | Up to about six lines inline in `mise.toml`; longer bodies in a POSIX `sh` script under `scripts/` or `server/scripts/`. All five fit inline today; `check-compose` already calls `scripts/compose-config-check.sh`. | Evidence: #496 body (Shared decisions), `justfile` |
| D3 | Do contributors still install `just`? | `just` stays in `mise.toml` and in the hand-install steps, because the fixture, `dev`, `e2e` and other recipes still need it until #500. | Evidence: `justfile`, #499 body (Scope, out) |
| D4 | When one part of the gate fails, does `mise run check` stop or run every part? | Stop at the first failure, and run the parts in order: Compose, OpenSpec, server, web, as `just check` does today. | Assumption |
| D5 | Does the `compose` CI job switch to `mise run check-compose` too? | Yes. All four check jobs run `mise run <task>`: `compose`, `openspec`, `server` and `web`. The `e2e` and `lets-spec` jobs and the fixture steps do not change. | Assumption |
| D6 | Does this change update every other mention of a gate command? | Yes, to its `mise run` form: the design-tokens, external-postgres and dev-environment spec scenarios, `openspec/config.yaml` context, `openspec/discovery.md`, `docs/security-scanning.md`, and the `check-compose` part of the usage message in `scripts/external-db-verify.sh`. `just external-db-verify` stays for #500. | Assumption |
| D7 | The Nix shell has no mise. What happens to the dev-environment gate requirement? | Accept the gap until #502 removes the Nix route. The requirement "The repo checks pass inside the shell" and its scenario name `mise run check` instead of `nix develop --command just check`. | Assumption |
| D8 | Once the gate recipes leave the `justfile`, how does a contributor who uses neither mise nor Nix run the gate? | `CONTRIBUTING.md`'s "Without Nix" steps list the commands of the four parts in order: Compose, OpenSpec, server, web. They are the commands the mise tasks run. The dev-environment scenario "A contributor without Nix runs the checks" runs those commands instead of `just check`. No extra script, so the gate is defined only in `mise.toml`. | Assumption |

## Changes from the story

- Added: the `compose` CI job switches to `mise run check-compose` (D5). The story's scope named only the `server`, `web` and `openspec` jobs; leaving `compose` on `just check-compose` would break once the recipe leaves the `justfile`.
- Added: the other spec scenarios, `openspec/config.yaml`, `openspec/discovery.md`, `docs/security-scanning.md` and `scripts/external-db-verify.sh` change with the docs (D6), because each would name a command that no longer exists.
- Added: the dev-environment Nix-shell gate requirement changes to `mise run check` (D7).
- Added: `CONTRIBUTING.md` lists the four gate commands in order for contributors without mise or Nix, and the dev-environment scenario for them changes to match (D8).

lets-go confirmed this ledger under the user's standing instruction to stop in the brainstorm only when a question has no recommendation; the user did not read it in person.
Confirmed by user: yes
