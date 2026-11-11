---
name: lets-plan
description: Plan one claimed change in an OpenSpec project, on the change's branch. Runs openspec-propose for the change, or continues it, and stops when the plan is ready. Use when lets-go runs the plan step, or when someone asks to plan or carry on planning the change on the current branch.
---

# Let's plan

Plan one change, on its branch. This is the plan step. `lets-go` claims
the issue and routes it first. Run every command below from the repo
root.

## Find where you are

1. Find the default branch:
   `gh repo view --json defaultBranchRef --jq .defaultBranchRef.name`.
2. Run `git branch --show-current`. On the default branch, say that
   `lets-go` starts an issue, and stop. Otherwise the branch is the
   change's name, `<name>`.
3. Take the issue, `<i>`, and the route line from the caller. When the
   caller gives no issue, search for it:

   ```bash
   gh issue list --state open --search "<branch> in:title" --json number,title
   ```

   Keep the one whose title starts with `<branch> — `. If none or
   several match, say that `lets-plan` needs the issue number, and stop.

## Plan

Invoke the `openspec-propose` skill for the change `<name>`, for issue
#<i>. Give it the route line for the proposal's commit message. When the
route is `lets-minimal` and the flow's schema is not, say to use the
schema `lets-minimal`. The flow's schema is the `schema:` line of
`openspec/config.yaml`.

When `openspec/changes/<name>/` exists, say to continue the change. When
it does not, and you have no route line, stop: say that `lets-go` routes
and claims the issue.

## When propose stops

Read the state:

```bash
openspec instructions apply --change <name> --json
```

- `ready`: say the plan is ready. On `lets-standard`, the plan review
  comes next.
- Anything else: say that propose stopped early, for example to ask a
  question, and stop.

## Rules

- Never plan on the default branch.
- Never build, and offer no handoff.
- Write no artifact yourself. The skills you invoke write them.
