## Why

Each OpenSpec change gets two independent reviews: a plan review before tasks, and a code review after apply. Three skills share that work today, and they overlap. `critique` and `peer-code-review` each decide which model family reviews. Plan review findings pass through two label sets before they become a verdict. `critique` also repeats checks that the `review` instruction in the schema already holds. Removing `critique` gives each review one home and leaves one skill that decides who reviews.

## What Changes

**Code review after apply**

- Add the `peer-code-review` skill. When every apply task is done, it has another model family's built-in reviewer review the branch, then fixes confirmed findings. Codex runs `codex review`, and Claude Code runs `/code-review`.
- Give `peer-code-review` a rules file that both reviewers load. It adds checks for change fit, tests, and simplicity.
- Make `peer-code-review` ask `choose-an-adversary` which family reviews, instead of keeping its own table.
- Run `peer-code-review` from the apply guidance in `openspec/config.yaml`. Name it in the workflow in `AGENTS.md`: propose, apply, review, verify, archive.

**Plan review in the schema**

- Remove the `critique` skill.
- Move `critique`'s short review method into the `review` instruction in `openspec/schemas/spec-driven-review/schema.yaml`. The method replaces the lines that call `critique` and convert its labels.
- Have the `review` instruction pick its reviewer through `choose-an-adversary` directly.
- Drop the stale mention of a `test-plan` artifact, which the schema does not have.

**Choosing the reviewer**

- Keep `choose-an-adversary` as the one place that picks the reviewing family and runs it read-only.
- Move `critique`'s rule for small reviews into it: a small, single-unit review may run in the same session, and the result must say it was not independent.

**Not in scope**

- Any change to `review.md`. Its template and the verdict rules in the schema stay as they are.
- Reviews outside OpenSpec. `critique` also fired when someone wrote a skill or a design, or when a subagent claimed its work was done. Those triggers go away. Anyone can still ask for a review by hand.
- The global copies of these skills outside this repo.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `dev-environment`: two scenarios name `critique` as an example of an authored skill. Replace it with a skill that still exists, in "Git tracks authored skills and the skill settings" and "Every supported agent sees the same skills".

## Impact

- **Skills:** `.agents/skills/critique/` leaves the repo. `peer-code-review` is new. `choose-an-adversary` changes.
- **Workflow:** `openspec/schemas/spec-driven-review/schema.yaml`, `openspec/config.yaml`, and `AGENTS.md` change. Its links `CLAUDE.md` and `GEMINI.md` change with it.
- **Docs:** `CONTRIBUTING.md` names `critique` as an example of an authored skill, and changes to name another.
- **Artifacts:** `review.md` keeps the same sections and the same `VERDICT:` and `CHANGES_APPLIED:` lines. Archived changes are not touched.
- **Cost:** each change now runs one more cross-model review, after apply. It takes several minutes and costs real tokens.
