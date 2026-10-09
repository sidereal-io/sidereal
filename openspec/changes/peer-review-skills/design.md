## Context

See proposal.md for why. Today the review work sits in four places:

- **`critique`** holds a method for attacking a plan, and decides between another family and an in-session review.
- **`choose-an-adversary`** finds the author's model family, picks a different one, and runs it read-only through a prompt.
- **The `review` instruction** in `openspec/schemas/spec-driven-review/schema.yaml` calls `critique`, adds checks for this workflow, and turns `critique`'s labels into review.md's verdict.
- **`peer-code-review`**, added on this branch, keeps its own table of reviewer families and runs a built-in reviewer.

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
- Reviews outside an OpenSpec change. Nothing fires on its own when someone writes a skill or a design.
- Changing the copies of these skills that live outside this repo.

## Decisions

### Plan review stays an artifact, and code review is a step after apply

The `review` artifact stays where it is, because it must block `tasks`. Code review runs from the apply guidance in `openspec/config.yaml`, once every task is done. Its fixes land as commits, and its findings go in the pull request under **Code review**.

*Alternative: a `code-review` artifact.* It would have to depend on `tasks`, so OpenSpec would mark it ready before any code exists. `openspec-ff-change` would then write it against code that is not there yet. Artifacts plan the work; they do not follow it.

### The plan review method moves into the schema, and `critique` goes

The `review` instruction gains `critique`'s method in a few lines:

1. Name the decision and the options it rules out.
2. Find the specific failure: the input, the scale, the partial failure.
3. Check the assumptions the repo can settle, and name the ones that bet on the future.
4. Find where the design stops working.

These lines replace the instruction's "run `critique`" and "map `critique`'s output" lines. The reviewer reports in review.md's own labels (Critical, Moderate, Suggestion), so nothing needs converting. The instruction stays about the same length.

*Alternative: a `peer-plan-review` skill.* It would only move text out of the schema, and the review would still need the schema's verdict rules. *Alternative: keep `critique` as a shared method.* Its only remaining caller would be the schema, and its own triggers would keep firing outside OpenSpec.

### `choose-an-adversary` picks the family; each caller decides how it runs

`choose-an-adversary` keeps three jobs:

- find the author's family;
- pick the first other family that is installed, in the order `gpt`, `claude`, `gemini`;
- fall back when no other family is installed.

It also gains `critique`'s rule for small reviews. A bounded, single-unit review with no new contracts may run in the same session, and the result must say it was not independent.

`peer-code-review` drops its own family table and asks `choose-an-adversary` instead. It keeps its two built-in review commands, because it is their only user. When the chosen family is `gemini`, which has no built-in reviewer, `peer-code-review` runs `agy` with a prompt. The prompt names the branch diff and includes the rules file.

*Alternative: move the built-in commands into `choose-an-adversary`.* With one caller, that adds a mode to the shared skill for no reuse.

### One rules file for code review, named `CLAUDE.md`

`peer-code-review/rules/CLAUDE.md` stays as it is. Each tool loads it by its own route:

| Tool | How the rules arrive |
|---|---|
| `codex review` | `-c developer_instructions=...` |
| `claude /code-review` | `--add-dir` on the rules folder, with `CLAUDE_CODE_ADDITIONAL_DIRECTORIES_CLAUDE_MD=1` |
| `agy` | Added to the prompt |

The file is named `CLAUDE.md` because that is the only way Claude's `/code-review` accepts extra rules. Plan review needs no rules file: it builds its own prompt from the schema's instruction.

## Risks / Trade-offs

- **The review tools change their flags.** `codex review` and `/code-review` are young, and their options may move. → `peer-code-review` treats an error like a missing tool: it tries the next family, and it reports any substitution.
- **No automatic review for skills and designs written outside a change.** `critique` used to catch those. → Anyone can still ask `choose-an-adversary` for a review by hand. Skill changes that go through an OpenSpec change still get a plan review and a code review.
- **Claude may load the rules file as project memory.** Claude Code reads a `CLAUDE.md` it finds in a folder it opens. → The file holds only review rules, about 30 lines. It is harmless if loaded.
- **This change is reviewed by the method it removes.** `critique` still exists when this change's plan review runs. → That review runs once. The next change uses the new instruction.
- **Each change costs one more cross-model review.** A code review takes several minutes and real tokens. → `peer-code-review` runs once per branch, and again only after a rework that changes behavior.
