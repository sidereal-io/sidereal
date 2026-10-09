## Context

See proposal.md for why. Today the review work sits in four places:

- **`critique`** holds a method for attacking a plan, and decides between another family and an in-session review.
- **`choose-an-adversary`** finds the author's model family, picks a different one, and runs it read-only through a prompt.
- **The `review` instruction** in `openspec/schemas/spec-driven-review/schema.yaml` calls `critique`, adds checks for this workflow, and turns `critique`'s labels into review.md's verdict.
- **`peer-code-review`**, added on this branch, keeps its own table of reviewer families and runs a built-in reviewer. It passes its rules as a `CLAUDE.md` in an added folder for Claude, and as `developer_instructions` for Codex.

Three command-line tools can run a review. Only two have a built-in code reviewer:

| Family | Tool | Built-in code reviewer |
|---|---|---|
| `gpt` | `codex` | `codex review --base <branch>` |
| `claude` | `claude` | `/code-review` |
| `gemini` | `agy` | None. It only runs a prompt. |

## Goals / Non-Goals

**Goals:**

- Give each review one home: plan review in the schema, code review in `peer-code-review`.
- Make `choose-an-adversary` the only place that picks the reviewing family.
- Let the repo add its own rules to each review.

**Non-Goals:**

- Changing review.md. Its template and the verdict rules in the schema stay word for word, including rounds, staleness, `CHANGES_APPLIED`, and rebuttals.
- Enforcing either review with a tool. Both reviews are instructions that agents follow, as the `review` artifact already is.
- Reviews outside an OpenSpec change. Nothing fires on its own when someone writes a skill or a design.
- Changing the copies of these skills that live outside this repo.

## Decisions

### Plan review stays an artifact, and code review is a step after apply

The `review` artifact stays where it is, because it must block `tasks`. Code review runs from the apply guidance in `openspec/config.yaml`, once every task is done. Its fixes land as commits, and its findings go in the pull request under **Code review**.

The apply guidance is advice that an agent can miss, so archive guidance adds a second check. Before archiving, the agent looks for the **Code review** section in the pull request. When it is missing, the agent runs `peer-code-review` first.

*Alternative: a `code-review` artifact.* It would have to depend on `tasks`, so OpenSpec would mark it ready before any code exists. `openspec-ff-change` would then write it against code that is not there yet. Artifacts plan the work; they do not follow it.

### The plan review method moves into the schema, and `critique` goes

The `review` instruction gains `critique`'s method in a few lines:

1. Name the decision and the options it rules out.
2. Find the specific failure: the input, the scale, the partial failure.
3. Check the assumptions the repo can settle, and name the ones that bet on the future.
4. Find where the design stops working.

These lines replace the instruction's "run `critique`" and "map `critique`'s output" lines. The reviewer reports in review.md's own labels. It marks every finding that blocks acceptance Critical, as the template's "Critical (blocking)" heading says. Moderate and Suggestion findings do not block. Nothing needs converting, and the instruction stays about the same length.

*Alternative: a `peer-plan-review` skill.* It would only move text out of the schema, and the review would still need the schema's verdict rules. *Alternative: keep `critique` as a shared method.* Its only remaining caller would be the schema, and its own triggers would keep firing outside OpenSpec.

### `choose-an-adversary` picks the family; each caller decides how it runs

`choose-an-adversary` keeps three jobs:

- find the author's family;
- pick the first other family that is installed, in the order `gpt`, `claude`, `gemini`;
- fall back when no other family is installed.

A review runs in the same session only when no other family is installed, and the result says it was not independent. The `review` artifact is the exception: it must not be written in the context that wrote the plan, so a fresh-context subagent writes it, and its metadata says so.

*Alternative: keep `critique`'s rule for small reviews,* which let a bounded, single-unit review run in the same session. It spared small edits outside OpenSpec from a cross-model run. Those triggers go away, and the rule's only remaining effect would be to let a small change skip its cross-model code review.

`peer-code-review` drops its own family table and asks `choose-an-adversary` instead. It keeps its two built-in review commands, because it is their only user. When the chosen family is `gemini`, which has no built-in reviewer, `peer-code-review` runs `agy` with a prompt. The prompt names the branch diff and tells the reviewer to apply `REVIEW.md`.

*Alternative: move the built-in commands into `choose-an-adversary`.* With one caller, that adds a mode to the shared skill for no reuse.

### Review rules live in a root `REVIEW.md`, and `AGENTS.md` points to it

The rules for change fit, tests, and simplicity move to `REVIEW.md` at the repo root. `AGENTS.md` gains one line: "When you review code, also apply the review rules in `REVIEW.md`." `CLAUDE.md` stays a link to `AGENTS.md`. Each reviewer runs read-only and takes no rules on its command line:

| Tool | Command | Read-only setting |
|---|---|---|
| Codex | `codex -s read-only review --base <base>` | `-s read-only` |
| Claude Code | `claude -p "/code-review high <base>...HEAD" --permission-mode plan` | `--permission-mode plan` |
| `agy` | a prompt that names the diff and `REVIEW.md` | `--mode plan` |

A test on a throwaway repo chose this route. Its root review rules asked for a marked finding on one changed line, and each route was run against that diff:

| Route | Rule applied |
|---|---|
| Codex: pointer in `AGENTS.md` | Yes |
| Claude: pointer in `AGENTS.md`, read through the `CLAUDE.md` link | Yes |
| Claude: `@REVIEW.md` import in `CLAUDE.md` | Yes, but every Claude session loads the rules |
| Claude: `CLAUDE.md` in a folder added with `--add-dir` (this branch today) | No. Claude scopes a folder's `CLAUDE.md` to files in that folder. |
| Claude: `--append-system-prompt-file` | No |
| Claude: `REVIEW.md` alone | No. The local `/code-review` does not read it; only the GitHub app does. |

The pointer costs other sessions one sentence. It names code review, so plan reviews and ordinary coding sessions leave the rules unread.

## Risks / Trade-offs

- **A reviewer skips the pointer.** The test ran each route once. → Both reviewers read `AGENTS.md` as project context, which their documentation describes. If a later review ignores the rules, Claude switches to the `@REVIEW.md` import, which the test showed works.
- **The review tools change their flags.** `codex review` and `/code-review` are young, and their options may move. → `peer-code-review` treats an error like a missing tool: it tries the next family, and it reports any substitution.
- **No automatic review for skills and designs written outside a change.** `critique` used to catch those. → Anyone can still ask `choose-an-adversary` for a review by hand. Skill changes that go through an OpenSpec change still get a plan review and a code review.
- **This change is reviewed by the method it removes.** `critique` still exists when this change's plan review runs. → That review runs once. The next change uses the new instruction.
- **Each change costs one more cross-model review.** A code review takes several minutes and real tokens. → `peer-code-review` runs once per branch, and again only after a rework that changes behavior.
