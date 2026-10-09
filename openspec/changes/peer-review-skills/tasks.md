## 1. Review rules

- [x] 1.1 Move the rules from `.agents/skills/peer-code-review/rules/CLAUDE.md` to a new `REVIEW.md` at the repo root, and delete the `rules/` folder. Keep the rules' wording. Verify: `git ls-files .agents/skills/peer-code-review` lists only `SKILL.md`, and `REVIEW.md` holds the change fit, tests, and simplicity sections.
- [x] 1.2 Add one line to `AGENTS.md`'s workflow section: "When you review code, also apply the review rules in `REVIEW.md`." Verify: `grep -n "REVIEW.md" AGENTS.md` finds the line, and `CLAUDE.md` is still a link to `AGENTS.md`.

## 2. Choosing the reviewer

- [ ] 2.1 Update `.agents/skills/choose-an-adversary/SKILL.md`. Drop "critique skills" from its description and body. Add the small-review rule, with its exception for the `review` artifact and the fresh-context subagent fallback, as design.md describes. Verify: `grep -n critique .agents/skills/choose-an-adversary/SKILL.md` finds nothing, and the skill states the rule and the exception.

## 3. Code review

- [ ] 3.1 Update `.agents/skills/peer-code-review/SKILL.md`. Replace its family table with a step that asks `choose-an-adversary`. Use the three commands in design.md's table, with no rules flags. Make the `agy` prompt name the branch diff and `REVIEW.md`. Verify: the skill contains `codex -s read-only review --base`, and `grep -nE "add-dir|developer_instructions|ADDITIONAL_DIRECTORIES|rules/" .agents/skills/peer-code-review/SKILL.md` finds nothing.
- [ ] 3.2 Add archive guidance to `openspec/config.yaml`: before archiving, check the pull request for a **Code review** section, and run `peer-code-review` first when it is missing. Verify: `openspec instructions archive --change peer-review-skills --json` lists the entry under `operationGuidance`.

## 4. Plan review in the schema

- [ ] 4.1 In the `review` instruction in `openspec/schemas/spec-driven-review/schema.yaml`, replace the lines that call `critique` and map its output. Add the four-step method, the step that picks the reviewer through `choose-an-adversary`, the fresh-context subagent fallback, and the rule that every blocking finding is Critical. Drop the stale `test-plan` mentions. Change no other verdict rule. Verify: `git grep -n -E "critique|test-plan" -- openspec/schemas` finds nothing, `git diff main -- openspec/schemas/spec-driven-review/templates/review.md` is empty, and the diff of `schema.yaml` touches only the `review` instruction.

## 5. Remove `critique`

- [ ] 5.1 Delete `.agents/skills/critique/`, and name `peer-code-review` instead of `critique` as the example of an authored skill in `CONTRIBUTING.md`. Verify: `git grep -n critique -- ':!openspec/changes' ':!openspec/specs/dev-environment'` finds nothing. The archive step updates the `dev-environment` spec.
- [ ] 5.2 Check the `dev-environment` delta scenarios. Verify: after `just skills`, `git status --porcelain` prints nothing, `.claude/skills/peer-code-review/SKILL.md` exists, and a second `just skills` changes no file under `.agents/skills`.

## 6. Final checks

- [ ] 6.1 Validate the change. Verify: `openspec validate peer-review-skills --strict` passes.
