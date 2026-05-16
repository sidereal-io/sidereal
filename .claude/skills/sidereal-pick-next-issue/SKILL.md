---
name: sidereal-pick-next-issue
description: Use when the user asks "what's next?", "what should I work on?", "find me an issue", "pick something to do", or starts a workflow without naming a specific issue. Surveys the GitHub issue queue, recommends the highest-priority `spec/ready` issue, and lists alternatives plus the `spec/brainstorming` queue.
---

# Pick the Next Issue

Help the user choose what to work on next by surveying the GitHub issue queue and recommending one issue, with alternatives.

## When to invoke

- User asks "what's next?", "what should I work on?", "anything to do?"
- User asks to start the workflow without naming an issue ("let's do the next one", "start something")
- User invokes `sidereal-workflow-overrides` or superpowers brainstorming/writing-plans without specifying an issue number

## Process

### Step 1 — Query both queues in parallel

```bash
gh issue list --label spec/ready          --json number,title,labels,createdAt,url --limit 50
gh issue list --label spec/brainstorming  --json number,title,labels,createdAt,url --limit 50
```

### Step 2 — Rank `spec/ready` candidates

Sort by priority label, then by age (oldest first within a tier):

1. `priority:high`
2. `priority:medium`
3. `priority:low`
4. (no priority label)

Within each tier, oldest `createdAt` wins.

### Step 3 — Present results

Format the response as three sections. Keep it scannable.

```markdown
**Recommended:** #<n> — <title>
Priority: <label or "unlabeled"> · Opened <relative date> · <url>

<one-sentence reason for the recommendation, e.g. "highest priority and oldest in the spec/ready queue">

**Other spec/ready issues:**
- #<n> — <title> (priority:<x>, opened <date>)
- #<n> — <title> (...)

**Brainstorming queue (needs refinement before implementation):**
- #<n> — <title> (opened <date>)
- #<n> — <title> (...)
```

If a section is empty, say so explicitly ("No issues in the brainstorming queue.") rather than omitting the heading.

### Step 4 — Hand off

Ask: "Want me to start on #<n>, or pick a different one?"

If the user picks an `spec/ready` issue:
- Invoke `sidereal-workflow-overrides` and start at Override 2 (branch creation), since the issue is already approved.

If the user picks a `spec/brainstorming` issue:
- Invoke `superpowers:brainstorming` with `sidereal-workflow-overrides` layered on top, starting at Override 1 (continue refining the issue body).

If both queues are empty:
- Report that. Suggest the user open a new issue with the `spec/brainstorming` label to start a fresh design.

## Edge cases

- **`gh` not authenticated:** Report the error verbatim and suggest `gh auth login`. Do not try to recover.
- **An issue has both `spec/brainstorming` and `spec/ready` labels:** Treat as `spec/ready` (the human-applied label wins) and flag the inconsistency to the user.
- **Issue assigned to someone else:** Still surface it but note the assignee. Don't filter it out — the user may be coordinating.
