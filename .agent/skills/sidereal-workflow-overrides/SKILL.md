---
name: sidereal-workflow-overrides
description: Use whenever the superpowers brainstorming, writing-plans, or finishing-a-development-branch skills would apply in the sidereal repo. Defines four project-specific overrides on top of the standard superpowers workflow — spec lives on a GitHub issue, plans live in `.workspace/plans/`, an ADR may be required before PR, and the PR body must close the issue.
---

# Sidereal Workflow Overrides

This skill is an **overlay** on the superpowers workflow. Follow the standard `superpowers:brainstorming`, `superpowers:writing-plans`, `superpowers:executing-plans` (or `subagent-driven-development`), and `superpowers:finishing-a-development-branch` skills as written, with the four overrides below.

If a step here conflicts with a superpowers skill, this skill wins (per superpowers' own "user instructions take precedence" rule).

## Override 1 — Spec lives on a GitHub issue

**Replaces:** `spec/brainstorming` Step 6 ("Write design doc to `docs/superpowers/specs/...`") and Step 8 ("User reviews the spec file").

In sidereal, the spec is the GitHub issue body, not a local file. The brainstorming dialogue, design proposals, and final spec all land on the issue.

```bash
# New idea
gh issue create --title "<title>" --label spec/brainstorming --body "<spec body>"

# Existing issue
gh issue edit <number> --add-label spec/brainstorming
gh issue edit <number> --body-file -   # update body from stdin
```

The user iterates on the issue. **Do not proceed to implementation until a human applies the `spec/ready` label** — never set this label yourself. Visual mockups belong as issue attachments.

The "user reviews the spec" gate becomes: "user applies `spec/ready` label."

## Override 2 — Branch naming and plan location

**Adds:** a step between `spec/brainstorming` and `writing-plans`.

Once the issue is `spec/ready`:

```bash
gh issue list --label spec/ready    # confirm what's ready
git fetch origin
git checkout -b issue-<number>-<short-slug> origin/main
```

**Replaces:** `writing-plans` save location.

Save the plan to `.workspace/plans/YYYY-MM-DD-issue-<number>-<slug>.md` (gitignored — plans are working documents). Include the issue URL at the top of the plan, immediately after the required superpowers header block.

## Override 3 — Verification cadence

**Adds:** to the per-task TDD loop.

After each task's test+commit cycle, run `npm run check` (TypeScript). Fix type errors before continuing. If the task touched `packages/shared/`, also run `npm run test`.

This is in addition to — not instead of — the Red-Green-Refactor loop.

## Override 4 — ADR review and PR body

**Adds:** an ADR step before `finishing-a-development-branch` Step 4.

Before presenting the finish menu, compare the implementation to the issue spec. If a notable design decision was made or changed during implementation — something a future contributor would need to understand — write an ADR using `docs/decisions/ADR-000-template.md` as the template. Number sequentially (`ADR-NNN-short-slug.md`). Set status to Proposed unless the user has already accepted it; if the PR implements an Accepted ADR, flip status in the same PR.

"Notable" = a trade-off where a reasonable person could argue for a different approach. Routine implementation choices don't need an ADR.

**Replaces:** `finishing-a-development-branch` Option 2 PR body template.

The PR body MUST include `Closes #<issue-number>` so the issue auto-closes on merge:

```bash
gh pr create --title "<type>: <short description>" --body "$(cat <<'EOF'
Closes #<number>

## Summary
<2-3 bullets of what changed>

## Test Plan
- [ ] <verification steps>
EOF
)"
```

Use Conventional Commits prefix in the title (`feat:`, `fix:`, `docs:`, `refactor:`, `test:`).

## What this skill does NOT change

Everything else in superpowers stays as written: the brainstorming dialogue style, the writing-plans header and task structure, the Red-Green-Refactor loop, frequent commits, the finishing-a-development-branch options menu, the test-must-pass gate. Only the four points above are overridden.

## Quick reference

| Phase | Standard superpowers | Sidereal override |
|---|---|---|
| Spec | File in `docs/superpowers/specs/` | GitHub issue body, `spec/brainstorming` label |
| Approval gate | User reviews spec file | Human applies `spec/ready` label |
| Branch | (not enforced) | `issue-<n>-<slug>` from `origin/main` |
| Plan | `docs/superpowers/plans/...` | `.workspace/plans/YYYY-MM-DD-issue-<n>-<slug>.md` |
| Per-task | TDD + commit | TDD + commit + `npm run check` |
| Pre-PR | (none) | ADR if notable decision |
| PR body | Summary + Test Plan | `Closes #<n>` + Summary + Test Plan |
