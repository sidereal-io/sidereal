---
name: peer-code-review
description: Use when an implementation branch is finished and needs a code review before verify, archive, or a pull request — such as when an OpenSpec change's apply tasks are all done. Also use when someone asks for a code review of a branch's changes.
---

# Peer Code Review

Have a different model family review the branch's code. A reviewer from the author's family shares the author's blind spots. Each family's own built-in reviewer runs the review where one exists, so this skill only picks the family, runs it read-only, and acts on what it finds.

## When to apply

- **After apply.** Every task in an OpenSpec change is done. Review before `openspec-verify-change`, so verify checks the fixed code. This is the one review per change.
- **On request.** Someone asks for a code review of a branch.

Archive only checks that this review happened and still covers the code. It does not start a second review on its own.

## Find the author

`<base>` is the branch this one merges into: the pull request's base (`gh pr view --json baseRefName`), else the parent branch of a stacked story, else `main`.

Find who wrote the code. First answer wins:

- **The commits.** Every family named in their `Co-Authored-By` lines is an author:

  ```bash
  git log --format='%(trailers:key=Co-Authored-By,valueonly)' <base>..HEAD | sort -u
  ```

- **This session** — the commits name no model, and you wrote the code here.
- **Ask the author.** Do not guess. A wrong answer turns the review into a rubber stamp.

Pick the reviewer from a family that wrote none of the commits.

## Pick the reviewer

| Author | Reviewer | Command |
|---|---|---|
| `claude` | `gpt` | `codex -s read-only review` |
| `gpt` | `claude` | `claude -p "/code-review ..." --permission-mode plan` |
| `gemini` | `gpt` | `codex -s read-only review` |

When the reviewer's tool is missing (`command -v`) or fails, try the next family that wrote none of the commits: `gpt`, then `claude`, then `gemini` through `agy`. Report any substitution.

When no other family is installed, run the author's own family's command below as a new process, so the review starts from a fresh context. Say plainly that the review was not independent. Never review in the author's own session.

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

This costs real tokens. Run it once per change, never to chase a clean result.

## Act on the findings

The reviewer has no product context, so some findings will miss. Check each one against the code before you accept it:

- **CONFIRMED** — the defect is real. Fix it.
- **REFUTED** — name the evidence that contradicts it.
- **OUT-OF-SCOPE** — real, but not this change's. Say why, and file an issue if it matters.

Fix each confirmed finding in its own commit with its real type (`fix:`, `refactor:`, `test:`). A fix commit changes only what its finding names. Put any other change in its own commit: it is rework, and it needs a new review. Then run `just check` if the fixes touched `server/` or `web/`.

## Report

List every finding with its verdict and, when confirmed, its fix commit, most severe first. Name the reviewer, the commit it reviewed, and whether the review was independent. Put the same list in the pull request description under **Code review**, so the person who merges sees what was found and refuted.

## Before archive

Archive guidance checks the **Code review** section. Run this skill again only when:

- the section is missing; or
- a commit after the reviewed one changes files outside `openspec/changes/` and is not a listed fix.

Before treating a listed fix commit as exempt, read its diff with `git show <sha>`. Confirm it touches only what its finding names. If it touches anything else, run the review again.
