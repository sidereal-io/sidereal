---
name: lets-build
description: Build one planned change in an OpenSpec project, on the change's branch. Runs openspec-apply-change, test-first, and stops when every task is ticked. Use when lets-go runs the build step, or when someone asks to build or carry on building the planned change on the current branch. To start an issue or the next story, use lets-go.
---

# Let's build

Build one planned change, on its branch. This is the build step.
`lets-go` claims the issue and plans it first, and `lets-review-build`
reviews and finishes the change afterward. Run every command below from the
repo root.

Read [Plain language](../lets-shape/references/plain-language.md) before
writing.
Use it for progress and stop messages: name the state, what is unfinished
and the next action.

## Find where you are

1. Find the default branch:
   `gh repo view --json defaultBranchRef --jq .defaultBranchRef.name`.
2. Run `git branch --show-current`.
3. If the branch has a folder `openspec/changes/<branch>/`, read its state:
   `openspec instructions apply --change <branch> --json`.
   If the command fails, or prints no JSON object with a `state`, show
   what it printed, and stop. Do not guess a state or retry.
4. Use the table from top to bottom. Follow the first matching row;
   later rows do not override it:

| The current branch | `state` | Do |
|---|---|---|
| is the default branch | — | Say that `lets-go` starts an issue, and stop. |
| has a change folder | `ready` | [Apply](#apply). |
| has a change folder | `blocked` | Say that the plan is not finished: run the `lets-plan` skill. Stop. |
| has a change folder | `all_done` | Say that every task is ticked, and that the `lets-review-build` skill finishes the change. Stop. |
| has a change folder | anything else | Show the user the CLI's `instruction`, and stop. When `instruction` is absent or unusable, show the full output instead. |
| has an archived folder whose whole name is `<date>-<branch>` in `openspec/changes/archive/` | — | Say the change is archived, and that its pull request shows whether it is ready. Stop. |
| anything else | — | Say that `lets-plan` plans a claimed change on its branch, and that `lets-go` starts an issue. Stop. |

## Apply

Invoke the `openspec-apply-change` skill for the change `<branch>`. The
schema's apply instruction works each task test-first with the
`lets-test-first` skill. When every task is ticked, stop, and say that
the `lets-review-build` skill finishes the change.

## Rules

- Build only. Never claim, plan, review or finish a change.
- Never write code or an artifact outside the skills you invoke.
- One change per run.
