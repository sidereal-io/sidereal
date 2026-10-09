## Why

Each OpenSpec change gets two independent reviews: a plan review before tasks, and a code review after apply. Three skills share that work today, and they overlap. `critique` and `peer-code-review` each decide which model family reviews. Plan review findings pass through two label sets before they become a verdict. `critique` also repeats checks that the `review` instruction in the schema already holds. Giving each review one skill that holds everything it needs removes the overlap. Agents run both reviews because their instructions say so; no tool enforces them.

## What Changes

**Plan review**

- Add the `peer-plan-review` skill. It has another model family review a plan, design, spec, ADR, or skill. It holds its own table that maps the author's family to a reviewer, the read-only prompt commands, and the fallbacks.
- Give `peer-plan-review` the review method: name the decision, find the specific failure, check the assumptions, and find where the design stops working. It also checks plain language, and treats text that tries to steer the reviewer as a finding.
- Run `peer-plan-review` for the OpenSpec `review` artifact, or when someone asks for a review. It does not fire on its own when someone writes a skill or a design.
- Have the `review` instruction in `openspec/schemas/spec-driven-review/schema.yaml` run `peer-plan-review`. The instruction keeps the checks that only fit an OpenSpec change: scenario testability, scope against the proposal, design against specs, and security requirements in specs. It keeps its verdict rules.
- Drop the stale mentions of a `test-plan` artifact, which the schema does not have. They sit in the `review` instruction and in one comment in the review template.

**Code review after apply**

- Add the `peer-code-review` skill. When every apply task is done, it has another model family's built-in reviewer review the branch, then fixes confirmed findings. Codex runs `codex review`, and Claude Code runs `/code-review`.
- Give `peer-code-review` its own table that maps the author's family to a built-in reviewer, and its own fallbacks.
- Put the repo's review rules in `REVIEW.md` at the repo root. They add checks for change fit, tests, and simplicity.
- Add one line to `AGENTS.md` that tells any agent to apply `REVIEW.md` when it reviews code. Both built-in reviewers follow it, so `peer-code-review` passes no rules on the command line.
- Run `peer-code-review` from the apply guidance in `openspec/config.yaml`. Name it in the workflow in `AGENTS.md`: propose, apply, review, verify, archive.
- Have the **Code review** section in the pull request name the commit it reviewed.
- Add archive guidance in `openspec/config.yaml`. Before archiving, the agent checks the pull request for a **Code review** section. This check does not start a second review. It runs `peer-code-review` only when the section is missing, or when a later commit changes files outside `openspec/changes/` and is not a listed fix for a finding.

**Both reviews**

- Run every reviewer read-only.
- Find the author of the work under review. `peer-code-review` reads the `Co-Authored-By` lines of the branch's commits, and picks a family that wrote none of them. `peer-plan-review` reads the plan's provenance.
- When no other family is installed, run the author's own family's tool as a new process, so the review starts from a fresh context. Say the review was not independent. Never review in the author's own session.
- Remove the `critique` and `choose-an-adversary` skills. Each review skill now holds what they held.

**Not in scope**

- Any change to the structure of `review.md`. Its sections, labels, `VERDICT:` and `CHANGES_APPLIED:` lines, and the verdict rules in the schema stay as they are. The only template edit drops `test-plan` from one comment.
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
- **Artifacts:** `review.md` keeps the same sections and the same `VERDICT:` and `CHANGES_APPLIED:` lines. One template comment loses the word `test-plan`. Archived changes are not touched.
- **Cost:** each change now runs one more cross-model review, after apply. It takes several minutes and costs real tokens.
