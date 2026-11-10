**Why:** Nothing runs OpenSpec validation before a merge, so a spec or change delta with errors or warnings can reach `main` unnoticed.

## Story

#481 openspec-validate-gate — just check fails on any OpenSpec validation error or warning, https://github.com/sidereal-io/sidereal/issues/481

## Decisions

| # | Question | Answer | Source |
|---|---|---|---|
| D1 | What fails the gate? | Any error or warning from `openspec validate --all --strict`; INFO notes do not fail it | Evidence: `openspec validate` output, OpenSpec 1.13.2 |
| D2 | What does it validate? | Every spec and every active change (`--all`), with `--no-interactive` | Evidence: `openspec validate --help` |
| D3 | What is the recipe called? | `check-openspec` | User |
| D4 | Where does it run in `just check`? | Second: `check: check-compose check-openspec check-server check-web` | User |
| D5 | How does CI run it? | A new `openspec` job whose one step, "OpenSpec gate", runs `nix develop --command just check-openspec`, using only the checkout and Nix actions | User |
| D6 | Which CLI runs it? | The development shell's `openspec`, pinned by `flake.lock`; no install step | Evidence: `openspec/specs/dev-environment/spec.md` |
| D7 | Does it send usage telemetry? | No: the recipe runs with `OPENSPEC_TELEMETRY=0` | Assumption |
| D8 | How much spec drift does this change fix? | Only the two stale `just --show check` scenarios in `dev-commands` and `ci` | User |

## Changes from the story

- Added: fix the stale `just --show check` scenarios in `dev-commands` and `ci`, because this change edits that recipe line.
- Split out: the `compose` and `lets-spec` jobs break the `ci` spec's one-recipe rule. Filed as #482.

Confirmed by user: yes
