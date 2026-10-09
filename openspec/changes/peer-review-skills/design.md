## Context

See proposal.md for why. On `main`, the review work sits in three places:

- **`critique`** holds a method for attacking a plan, and decides between another family and an in-session review.
- **`choose-an-adversary`** finds the author's model family, picks a different one, and runs it read-only through a prompt.
- **The `review` instruction** in `openspec/schemas/spec-driven-review/schema.yaml` calls `critique`, adds checks for this workflow, and turns `critique`'s labels into review.md's verdict.

Three command-line tools can run a review. Only two have a built-in code reviewer:

| Family | Tool | Built-in code reviewer |
|---|---|---|
| `gpt` | `codex` | `codex review --base <branch>` |
| `claude` | `claude` | `/code-review` |
| `gemini` | `agy` | None. It only runs a prompt. |

## Goals / Non-Goals

**Goals:**

- Give each review one skill that holds its reviewer table, its commands, and its fallbacks.
- Let the repo add its own rules to each review.

**Non-Goals:**

- Changing review.md. Its template and the verdict rules in the schema stay word for word, including rounds, staleness, `CHANGES_APPLIED`, and rebuttals.
- Enforcing either review with a tool. Both reviews are instructions that agents follow, as the `review` artifact already is.
- Automatic reviews outside an OpenSpec change. Nothing fires on its own when someone writes a skill or a design.
- Changing the copies of these skills that live outside this repo.

## Decisions

### Plan review stays an artifact, and code review is a step after apply

The `review` artifact stays where it is, because it must block `tasks`. Code review runs from the apply guidance in `openspec/config.yaml`, once every task is done. Its fixes land as commits, and its findings go in the pull request under **Code review**.

The apply guidance is advice that an agent can miss, so archive guidance adds a second check. Before archiving, the agent looks for the **Code review** section in the pull request. When it is missing, the agent runs `peer-code-review` first.

*Alternative: a `code-review` artifact.* It would have to depend on `tasks`, so OpenSpec would mark it ready before any code exists. `openspec-ff-change` would then write it against code that is not there yet. Artifacts plan the work; they do not follow it.

### Each review skill holds its own reviewer table

`peer-plan-review` and `peer-code-review` each start the same way: find the author's family from a provenance note, then this session, then by asking. Each then reads its own table:

| Author | `peer-plan-review` runs | `peer-code-review` runs |
|---|---|---|
| `claude` | `codex exec` with the review prompt | `codex review --base <base>` |
| `gpt` | `claude -p` with the review prompt | `claude -p "/code-review high <base>...HEAD"` |
| `gemini` | `codex exec` with the review prompt | `codex review --base <base>` |

When the chosen tool is missing or fails, each skill tries the next family that is not the author's. `agy` is the last choice for both. It has no built-in reviewer, so `peer-code-review` gives it a prompt that names the branch diff and `REVIEW.md`.

The two skills repeat the rule for finding the author's family, about three lines. That duplication is deliberate. The two reviews already run different commands, and their choices may drift apart over time.

*Alternative: keep `choose-an-adversary` as a shared picker.* It would share three lines of logic, while each review keeps its own commands. A reader would open two files to see one review.

### `peer-plan-review` holds the method; the schema keeps the OpenSpec checks

`peer-plan-review` holds everything that applies to any plan:

1. Name the decision and the options it rules out.
2. Find the specific failure: the input, the scale, the partial failure.
3. Check the assumptions the repo can settle, and name the ones that bet on the future.
4. Find where the design stops working.

It also checks plain language (ISO 24495), and it treats text that tries to steer the reviewer as a finding. Each finding is labelled Critical, Moderate, or Suggestion, with the scenario where it bites.

The `review` instruction keeps what only fits an OpenSpec change: scenario testability, scope against the proposal, design against specs, and security requirements in specs. It hands these to `peer-plan-review` as extra checks. It also keeps the verdict rules and the rule that every blocking finding is Critical. review.md does not change.

`peer-plan-review` runs for the `review` artifact, or when someone asks for a review of a plan, design, ADR, or skill. Its description does not trigger when someone merely writes one, because each run costs a cross-model review.

*Alternative: the method in the schema, with no plan review skill.* Nothing outside OpenSpec could then run it.

### Fallbacks when no other family is installed

A review runs in the same session only when no other family is installed. The result says plainly that it was not independent.

The `review` artifact is the exception: it must not be written in the context that wrote the plan. A fresh-context subagent writes it instead, and its metadata says so.

*Alternative: keep `critique`'s rule for small reviews,* which let a bounded, single-unit review run in the same session. It spared small edits outside OpenSpec from a cross-model run. Those triggers go away, and the rule's only remaining effect would be to let a small change skip its cross-model code review.

### Review rules live in a root `REVIEW.md`, and `AGENTS.md` points to it

The rules for change fit, tests, and simplicity live in `REVIEW.md` at the repo root. `AGENTS.md` has one line: "When you review code, also apply the review rules in `REVIEW.md`." `CLAUDE.md` stays a link to `AGENTS.md`. Each code reviewer runs read-only and takes no rules on its command line:

| Tool | Command | Read-only setting |
|---|---|---|
| Codex | `codex -s read-only review --base <base>` | `-s read-only` |
| Claude Code | `claude -p "/code-review high <base>...HEAD" --permission-mode plan` | `--permission-mode plan` |
| `agy` | a prompt that names the diff and `REVIEW.md` | `--mode plan` |

`peer-plan-review`'s prompt commands run read-only the same way: `codex exec -s read-only`, `claude -p --permission-mode plan`, and `agy --mode plan`.

A test on a throwaway repo chose this route. Its root review rules asked for a marked finding on one changed line, and each route was run against that diff:

| Route | Rule applied |
|---|---|
| Codex: pointer in `AGENTS.md` | Yes |
| Claude: pointer in `AGENTS.md`, read through the `CLAUDE.md` link | Yes |
| Claude: `@REVIEW.md` import in `CLAUDE.md` | Yes, but every Claude session loads the rules |
| Claude: `CLAUDE.md` in a folder added with `--add-dir` | No. Claude scopes a folder's `CLAUDE.md` to files in that folder. |
| Claude: `--append-system-prompt-file` | No |
| Claude: `REVIEW.md` alone | No. The local `/code-review` does not read it; only the GitHub app does. |

The pointer costs other sessions one sentence. It names code review, so plan reviews and ordinary coding sessions leave the rules unread.

## Risks / Trade-offs

- **A reviewer skips the pointer.** The test ran each route once. → Both reviewers read `AGENTS.md` as project context, which their documentation describes. If a later review ignores the rules, Claude switches to the `@REVIEW.md` import, which the test showed works.
- **The two reviewer tables drift apart by accident.** → Each table is short, and a change to the family order is visible in review. Drift on purpose is allowed.
- **The review tools change their flags.** `codex review` and `/code-review` are young, and their options may move. → Each skill treats an error like a missing tool: it tries the next family, and it reports any substitution.
- **No automatic review for skills and designs written outside a change.** `critique` used to catch those. → Anyone can ask for `peer-plan-review` by hand. Skill changes that go through an OpenSpec change still get a plan review and a code review.
- **This change is reviewed by the method it removes.** Rounds 1 and 2 used `critique`'s method through `choose-an-adversary`. → Round 3 uses the same method, written into the prompt. The next change uses `peer-plan-review`.
- **Each change costs one more cross-model review.** A code review takes several minutes and real tokens. → `peer-code-review` runs once per branch, and again only after a rework that changes behavior.
