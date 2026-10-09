---
name: cross-review
description: Use when an implementation branch is finished and needs a code review before verify, archive, or a pull request — such as when an OpenSpec change's apply tasks are all done. Also use when someone asks for a code review of a branch's changes.
---

# Cross-Review the Code

Have a different model family review the branch's code. A reviewer from the author's family shares the author's blind spots. Each family's own built-in reviewer runs the review, so this skill only picks one, runs it read-only, and acts on what it finds.

## When to apply

- **After apply.** Every task in an OpenSpec change is done. Review before `openspec-verify-change`, so verify checks the fixed code.
- **On request.** Someone asks for a code review of a branch.

## Pick the reviewer

Establish the author family as `choose-an-adversary` does: a provenance note, then this session, then ask. Never guess.

| Author family | Reviewer | Command |
|---|---|---|
| `claude` | Codex | `codex review --base <base>` |
| `gpt` | Claude Code | `claude -p --permission-mode plan "/code-review high <base>...HEAD"` |
| `gemini` | Codex | `codex review --base <base>` |

`<base>` is the branch this one merges into: the pull request's base (`gh pr view --json baseRefName`), else the parent branch of a stacked story, else `main`.

Commit or stash first. Both commands review commits, not the working tree.

Run from the repo root and write the output to `.workspace/cross-review-raw.md`. A review can take many minutes, so run it in the background with a long timeout.

Both reviewers read `AGENTS.md` (Claude through the `CLAUDE.md` link), so they see the repo's invariants without extra instructions. Neither accepts a custom checklist. `openspec-verify-change` checks that the code matches the change's artifacts.

**If the reviewer's executor is missing or fails** (`command -v`, an error, or no credit), run the other command only when its family is not the author's. Otherwise review in-session with the same method, and say plainly that the review was not independent.

This costs real tokens. Run it once per branch. Run it again only after a rework that changes behavior, never to chase a clean result.

## Act on the findings

The reviewer has no product context, so some findings will miss. Check each one against the code before you accept it:

- **CONFIRMED** — the defect is real. Fix it.
- **REFUTED** — name the evidence that contradicts it.
- **OUT-OF-SCOPE** — real, but not this change's. Say why, and file an issue if it matters.

Fix each confirmed finding in its own commit with its real type (`fix:`, `refactor:`, `test:`). Then run `just check` if the fixes touched `server/` or `web/`.

## Report

List every finding with its verdict, most severe first. Name the reviewer and say whether the review was independent. Put the same list in the pull request description under **Code review**, so the person who merges sees what was found and refuted.
