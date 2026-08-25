---
name: executing-plans
description: Execute an implementation plan (an ephemeral `.workspace` plan doc) step-by-step to a merged, closed-out slice. Use when the user says "execute/implement/continue the plan", points at a plan doc, or when `writing-plans` hands off. Fully resumable — a fresh session picks up from the plan file alone.
---

# Executing Plans

The plan doc is the single source of truth for execution state within its own scope — a fresh session resumes from the file alone. Harness task lists and conversation memory are never the record. Where a plan step was materialized as a GitHub sub-issue (per `writing-plans`), that sub-issue's open/closed state and comment trail are the record for that step instead, per `CLAUDE.md`'s Feature & Bug Workflow.

## Step 1 - Setup

Load the plan, review critically and determine whether a worktree is recommended — if the fix should only take a few minutes at most, a standard branch should suffice. Never commit on main.

If a worktree is recommended, prefer platform-native tools for creating and working in worktrees and fall back to manual git worktrees when no native tool is available.

If executing a sub-issue, comment on it that you've started (`gh issue comment <n> --body "Started."`).

Add under the plan header:

```markdown
## Execution state
- **Branch:** <name>
- **Workspace:** checkout | worktree <path>
- **Started:** <date>
- **Deviations:** (append one line per deviation, dated)
```

## Step 2 — Execute steps

- The orchestrator does briefs, integration, deviations, and gates; implementers execute briefs.
- The orchestrator should invoke parallel steps when possible.
- **Tick = evidence.** A checkbox is ticked only with its verification inline:

  ```markdown
  - [x] Step 3 — envelope adjudication
        ✓ npm run check && npm test -- src/rules/select.test.ts → 14 passed (2026-07-16)
  ```

  A ticked box without an evidence line is unticked. Never tick ahead; commit as you go.
- **Deviations:** append a dated line to the log; fix small plan errors in place, stop and surface material ones (changed guarantees, new migrations, authz surface). Never silently diverge.
- **Step review gates:** after a risk-bearing step or the smallest coherent batch, dispatch a fresh read-only reviewer (contract, correctness, test quality, simplification) before dependent work. Critical/Important findings block; verify each finding in source before fixing; fixes get a fresh re-review.
- If an **ADR-tier decision** is ever being made during this stage, STOP and surface it.

## Step 3 — Final slice gate

1. A **new** read-only `reviewer` subagent on the full branch diff — never the orchestrator reviewing itself, never a reused reviewer.
2. Verify every finding against source before applying; reject stale ones with reasons.
3. Verify the behavior through the narrowest automated seam (`npm run check`, targeted tests).
4. Verify all documentation is up-to-date with the new changes.
5. If executing a sub-issue, comment the commit/PR reference and close it (`gh issue comment <n> --body "Done in <sha>."`, `gh issue close <n>`).

## When to Stop and Ask for Help

STOP executing immediately when:

- Hit a blocker (missing dependency, test fails, instruction unclear)
- Plan has critical gaps preventing starting
- You don't understand an instruction
- Verification fails repeatedly
- Ask for clarification rather than guessing.

## Handoff

On completion of all steps, provide a report of what was completed, anything that was skipped and why, residue that was found, and recommendations for follow-up work. When every sub-issue under the parent is closed, open the PR (`Closes #<parent>`) and move the parent to `status/review`, per `CLAUDE.md`.
