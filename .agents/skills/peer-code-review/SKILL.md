---
name: peer-code-review
description: Use when an implementation branch is finished and needs a code review before verify, archive, or a pull request — such as when an OpenSpec change's apply tasks are all done. Also use when someone asks for a code review of a branch's changes.
---

# Peer Code Review

Have a different model family review the branch's code. A reviewer from the author's family shares the author's blind spots. Each family's own built-in reviewer runs the review where one exists, so this skill only picks the family, runs it read-only, and acts on what it finds.

## When to apply

- **After apply.** Every task in an OpenSpec change is done. Review before `openspec-verify-change`, so verify checks the fixed code.
- **Before archive.** The pull request has no **Code review** section, or later commits change files outside `openspec/changes/`.
- **On request.** Someone asks for a code review of a branch.

## Find the author

`<base>` is the branch this one merges into: the pull request's base (`gh pr view --json baseRefName`), else the parent branch of a stacked story, else `main`.

Read who wrote the code from its commits:

```bash
git log --format='%(trailers:key=Co-Authored-By,valueonly)' <base>..HEAD | sort -u
```

Every family named there is an author. Pick a reviewer from a family that wrote none of the commits. When the commits name no model, the author is this session's family. If you did not write them, ask. Do not guess: a wrong answer turns the review into a rubber stamp.

## Pick the reviewer

| Author | Reviewer | Command |
|---|---|---|
| `claude` | `gpt` | `codex -s read-only review` |
| `gpt` | `claude` | `claude -p "/code-review ..." --permission-mode plan` |
| `gemini` | `gpt` | `codex -s read-only review` |

When the reviewer's tool is missing (`command -v`) or fails, try the next family that wrote none of the commits: `gpt`, then `claude`, then `gemini` through `agy`. Report any substitution.

When no other family is installed, review in the same session against `REVIEW.md`, and say plainly that the review was not independent.

## Run the review

Commit or stash first. The commands review commits, not the working tree. Run from the repo root.

Every command is read-only, and none takes extra rules. The repo's review rules live in `REVIEW.md`, and `AGENTS.md` tells every reviewer to apply them. Claude reads `AGENTS.md` through the `CLAUDE.md` link. Name the output after the branch, so parallel sessions do not collide.

```bash
# gpt: Codex's built-in reviewer, in a read-only sandbox.
codex -s read-only review --base <base> > .workspace/<branch>-code-review-raw.md 2>&1

# claude: Claude Code's built-in reviewer, in plan mode.
claude -p "/code-review high <base>...HEAD" --permission-mode plan \
  < /dev/null > .workspace/<branch>-code-review-raw.md 2>&1

# gemini: no built-in reviewer, so run a prompt in plan mode.
agy --mode plan --model "<a gemini model from agy models>" --print-timeout <timeout> \
  -p "Review the changes in git diff <base>...HEAD for correctness bugs. Apply the review rules in REVIEW.md. For each finding, give the file, the line, the defect, and the scenario where it bites." \
  > .workspace/<branch>-code-review-raw.md 2>&1
```

Note the commit you reviewed: `git rev-parse --short HEAD`.

A review can take many minutes, so run it in the background with a long timeout. When `/code-review` finds nothing, its whole output is `(none)`. That is a clean result, not a failure.

This costs real tokens. Run it once per branch. Run it again only when later commits change files outside `openspec/changes/`, never to chase a clean result.

## Act on the findings

The reviewer has no product context, so some findings will miss. Check each one against the code before you accept it:

- **CONFIRMED** — the defect is real. Fix it.
- **REFUTED** — name the evidence that contradicts it.
- **OUT-OF-SCOPE** — real, but not this change's. Say why, and file an issue if it matters.

Fix each confirmed finding in its own commit with its real type (`fix:`, `refactor:`, `test:`). Then run `just check` if the fixes touched `server/` or `web/`.

## Report

List every finding with its verdict, most severe first. Name the reviewer, the commit it reviewed, and whether the review was independent. Put the same list in the pull request description under **Code review**, so the person who merges sees what was found and refuted.
