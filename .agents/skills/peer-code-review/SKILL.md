---
name: peer-code-review
description: Use when an implementation branch is finished and needs a code review before verify, archive, or a pull request — such as when an OpenSpec change's apply tasks are all done. Also use when someone asks for a code review of a branch's changes.
---

# Peer Code Review

Have a different model family review the branch's code. A reviewer from the author's family shares the author's blind spots. Each family's own built-in reviewer runs the review where one exists, so this skill only picks the family, runs it read-only, and acts on what it finds.

## When to apply

- **After apply.** Every task in an OpenSpec change is done. Review before `openspec-verify-change`, so verify checks the fixed code.
- **Before archive.** The pull request has no **Code review** section yet.
- **On request.** Someone asks for a code review of a branch.

## Pick the reviewer

Use `choose-an-adversary` to establish the author family and pick the reviewing family. It also covers the fallbacks: the next family when an executor is missing or fails, and an in-session review when no other family is installed.

## Run the review

Commit or stash first. The commands review commits, not the working tree.

`<base>` is the branch this one merges into: the pull request's base (`gh pr view --json baseRefName`), else the parent branch of a stacked story, else `main`.

Run from the repo root. Every command is read-only, and none takes extra rules. The repo's review rules live in `REVIEW.md`, and `AGENTS.md` tells every reviewer to apply them. Claude reads `AGENTS.md` through the `CLAUDE.md` link.

```bash
# gpt: Codex's built-in reviewer, in a read-only sandbox.
codex -s read-only review --base <base> > .workspace/peer-code-review-raw.md 2>&1

# claude: Claude Code's built-in reviewer, in plan mode.
claude -p "/code-review high <base>...HEAD" --permission-mode plan \
  < /dev/null > .workspace/peer-code-review-raw.md 2>&1

# gemini: no built-in reviewer, so run a prompt in plan mode.
agy --mode plan --model "<a gemini model from agy models>" --print-timeout <timeout> \
  -p "Review the changes in git diff <base>...HEAD for correctness bugs. Apply the review rules in REVIEW.md. For each finding, give the file, the line, the defect, and the scenario where it bites." \
  > .workspace/peer-code-review-raw.md 2>&1
```

A review can take many minutes, so run it in the background with a long timeout. When `/code-review` finds nothing, its whole output is `(none)`. That is a clean result, not a failure.

This costs real tokens. Run it once per branch. Run it again only after a rework that changes behavior, never to chase a clean result.

## Act on the findings

The reviewer has no product context, so some findings will miss. Check each one against the code before you accept it:

- **CONFIRMED** — the defect is real. Fix it.
- **REFUTED** — name the evidence that contradicts it.
- **OUT-OF-SCOPE** — real, but not this change's. Say why, and file an issue if it matters.

Fix each confirmed finding in its own commit with its real type (`fix:`, `refactor:`, `test:`). Then run `just check` if the fixes touched `server/` or `web/`.

## Report

List every finding with its verdict, most severe first. Name the reviewer and say whether the review was independent. Put the same list in the pull request description under **Code review**, so the person who merges sees what was found and refuted.
