---
name: peer-code-review
description: Use when an implementation branch is finished and needs a code review before verify, archive, or a pull request — such as when an OpenSpec change's apply tasks are all done. Also use when someone asks for a code review of a branch's changes.
---

# Peer Code Review

Have a different model family review the branch's code. A reviewer from the author's family shares the author's blind spots. Each family's own built-in reviewer runs the review, so this skill only picks one, runs it read-only, and acts on what it finds.

## When to apply

- **After apply.** Every task in an OpenSpec change is done. Review before `openspec-verify-change`, so verify checks the fixed code.
- **On request.** Someone asks for a code review of a branch.

## Pick the reviewer

Establish the author family as `choose-an-adversary` does: a provenance note, then this session, then ask. Never guess.

| Author family | Reviewer |
|---|---|
| `claude` | Codex |
| `gpt` | Claude Code |
| `gemini` | Codex |

## Run the review

Commit or stash first. Both commands review commits, not the working tree.

`<base>` is the branch this one merges into: the pull request's base (`gh pr view --json baseRefName`), else the parent branch of a stacked story, else `main`.

Run from the repo root. Both commands load the same extra rules from `rules/CLAUDE.md` in this skill's folder: change fit, tests, and simplicity. Both reviewers also read `AGENTS.md` for the repo's invariants.

```bash
rules=.agents/skills/peer-code-review/rules

# Codex. The rules arrive as developer instructions.
codex -c "developer_instructions=$(cat "$rules/CLAUDE.md")" review --base <base> \
  > .workspace/peer-code-review-raw.md 2>&1

# Claude Code. The rules arrive as a CLAUDE.md from an added folder.
# Keep the prompt before --add-dir, which would otherwise swallow it.
CLAUDE_CODE_ADDITIONAL_DIRECTORIES_CLAUDE_MD=1 \
  claude -p "/code-review high <base>...HEAD" --permission-mode plan --add-dir "$rules" \
  < /dev/null > .workspace/peer-code-review-raw.md 2>&1
```

A review can take many minutes, so run it in the background with a long timeout. When `/code-review` finds nothing, its whole output is `(none)`. That is a clean result, not a failure.

**If the reviewer's executor is missing or fails** (`command -v`, an error, or no credit), run the other command only when its family is not the author's. Otherwise review in-session against the rules file, and say plainly that the review was not independent.

This costs real tokens. Run it once per branch. Run it again only after a rework that changes behavior, never to chase a clean result.

## Act on the findings

The reviewer has no product context, so some findings will miss. Check each one against the code before you accept it:

- **CONFIRMED** — the defect is real. Fix it.
- **REFUTED** — name the evidence that contradicts it.
- **OUT-OF-SCOPE** — real, but not this change's. Say why, and file an issue if it matters.

Fix each confirmed finding in its own commit with its real type (`fix:`, `refactor:`, `test:`). Then run `just check` if the fixes touched `server/` or `web/`.

## Report

List every finding with its verdict, most severe first. Name the reviewer and say whether the review was independent. Put the same list in the pull request description under **Code review**, so the person who merges sees what was found and refuted.
