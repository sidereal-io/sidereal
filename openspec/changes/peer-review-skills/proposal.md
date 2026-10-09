## Why

Each OpenSpec change gets two independent reviews: a plan review before tasks, and a code review after apply. Three skills share that work today, and they overlap. `critique` and `peer-code-review` each decide which model family reviews. Plan review findings pass through two label sets before they become a verdict. `critique` also repeats checks that the `review` instruction in the schema already holds. Giving each review one skill that holds everything it needs removes the overlap. Agents run both reviews because their instructions say so; no tool enforces them.

## What Changes

**Plan review**

- Add the `peer-plan-review` skill. It has another model family review a plan, design, spec, ADR, or skill. It holds its own table that maps the author's family to a reviewer, the read-only prompt commands, and the fallbacks.
- Give `peer-plan-review` the review method: name the decision, find the specific failure, check the assumptions, and find where the design stops working. It also checks plain language, and treats text that tries to steer the reviewer as a finding.
- Run `peer-plan-review` for the OpenSpec `review` artifact, or when someone asks for a review. It does not fire on its own when someone writes a skill or a design.
- Have the `review` instruction in `openspec/schemas/spec-driven-review/schema.yaml` run `peer-plan-review`. The instruction keeps the checks that only fit an OpenSpec change: scenario testability, scope against the proposal, design against specs, and security requirements in specs. It keeps its verdict rules.
- Drop the stale mention of a `test-plan` artifact, which the schema does not have.

**Code review after apply**

- Add the `peer-code-review` skill. When every apply task is done, it has another model family's built-in reviewer review the branch, then fixes confirmed findings. Codex runs `codex review`, and Claude Code runs `/code-review`.
- Give `peer-code-review` its own table that maps the author's family to a built-in reviewer, and its own fallbacks.
- Put the repo's review rules in `REVIEW.md` at the repo root. They add checks for change fit, tests, and simplicity.
- Add one line to `AGENTS.md` that tells any agent to apply `REVIEW.md` when it reviews code. Both built-in reviewers follow it, so `peer-code-review` passes no rules on the command line.
- Run `peer-code-review` from the apply guidance in `openspec/config.yaml`. Name it in the workflow in `AGENTS.md`: propose, apply, review, verify, archive.
- Add archive guidance in `openspec/config.yaml`. Before archiving, the agent checks the pull request for a **Code review** section, and runs `peer-code-review` first when it is missing.

**Both reviews**

- Run every reviewer read-only.
- Review in the same session only when no other family is installed, and say the review was not independent. The `review` artifact uses a fresh-context subagent instead.
- Remove the `critique` and `choose-an-adversary` skills. Each review skill now holds what they held.

**Not in scope**

- Any change to `review.md`. Its template and the verdict rules in the schema stay as they are.
- Automatic reviews outside OpenSpec. `critique` fired when someone wrote a skill or a design, or when a subagent claimed its work was done. Those triggers go away. `peer-plan-review` runs on request.
- The global copies of these skills outside this repo.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `dev-environment`: two scenarios name `critique` and `choose-an-adversary` as examples of authored skills. Replace them with skills that still exist, in "Git tracks authored skills and the skill settings" and "Every supported agent sees the same skills".

## Impact

- **Skills:** `.agents/skills/critique/` and `.agents/skills/choose-an-adversary/` leave the repo. `peer-plan-review` and `peer-code-review` are new. `peer-code-review` loses the `rules/` folder it has on this branch today.
- **Review rules:** `REVIEW.md` is new at the repo root. Claude Code's GitHub app also reads it, if the repo ever turns that app on.
- **Workflow:** `openspec/schemas/spec-driven-review/schema.yaml`, `openspec/config.yaml`, and `AGENTS.md` change. Its links `CLAUDE.md` and `GEMINI.md` change with it.
- **Docs:** `CONTRIBUTING.md` names `critique` as an example of an authored skill, and changes to name another.
- **Artifacts:** `review.md` keeps the same sections and the same `VERDICT:` and `CHANGES_APPLIED:` lines. Archived changes are not touched.
- **Cost:** each change now runs one more cross-model review, after apply. It takes several minutes and costs real tokens.
