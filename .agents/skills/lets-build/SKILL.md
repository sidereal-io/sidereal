---
name: lets-build
description: Build one planned change in an OpenSpec project, on the change's branch. Runs openspec-apply-change, test-first, and stops when every task is ticked. Use when lets-go runs the build step, or when someone asks to build or carry on building the planned change on the current branch. To start an issue or the next story, use lets-go.
---

# Let's build

Build one planned change, on its branch. This is the build step.
`lets-go` claims the issue and plans it first, and `lets-review-build`
finishes the change after. Run every command below from the repo root.

## Find where you are

1. Find the default branch:
   `gh repo view --json defaultBranchRef --jq .defaultBranchRef.name`.
2. Run `git branch --show-current`.
3. If the branch has a folder `openspec/changes/<branch>/`, read its state:
   `openspec instructions apply --change <branch> --json`.
4. Take the first row that matches:

| The current branch | `state` | Do |
|---|---|---|
| is the default branch | — | Say that `lets-go` starts an issue, and stop. |
| has a change folder | `ready` | [Apply](#apply). |
| has a change folder | `blocked` | Say that the plan is not finished: run the `lets-plan` skill. Stop. |
| has a change folder | `all_done` | Say that every task is ticked, and that the `lets-review-build` skill finishes the change. Stop. |
| has a change folder | anything else | Show the user the CLI's `instruction`, and stop. |
| has an archived folder whose whole name is `<date>-<branch>` in `openspec/changes/archive/` | — | Say the change is archived and its pull request is ready for review. Stop. |
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
