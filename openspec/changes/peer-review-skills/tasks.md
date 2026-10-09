## 1. Review rules

- [x] 1.1 Move the rules from `.agents/skills/peer-code-review/rules/CLAUDE.md` to a new `REVIEW.md` at the repo root, and delete the `rules/` folder. Keep the rules' wording. Verify: `git ls-files .agents/skills/peer-code-review` lists only `SKILL.md`, and `REVIEW.md` holds the change fit, tests, and simplicity sections.
- [x] 1.2 Add one line to `AGENTS.md`'s workflow section: "When you review code, also apply the review rules in `REVIEW.md`." Verify: `grep -n "REVIEW.md" AGENTS.md` finds the line, and `CLAUDE.md` is still a link to `AGENTS.md`.

## 2. Choosing the reviewer

- [x] 2.1 Update `.agents/skills/choose-an-adversary/SKILL.md`. Drop "critique skills" from its description and body. Add the in-session fallback for when no other family is installed, with its exception for the `review` artifact (a fresh-context subagent), as design.md describes. Verify: `grep -n critique .agents/skills/choose-an-adversary/SKILL.md` finds nothing, and the skill states the fallback and the exception. Replaced by 7.5 after plan review round 3.

## 3. Code review

- [x] 3.1 Update `.agents/skills/peer-code-review/SKILL.md`. Replace its family table with a step that asks `choose-an-adversary`. Use the three commands in design.md's table, with no rules flags. Make the `agy` prompt name the branch diff and `REVIEW.md`. Verify: the skill contains `codex -s read-only review --base`, and `grep -nE "add-dir|developer_instructions|ADDITIONAL_DIRECTORIES|rules/" .agents/skills/peer-code-review/SKILL.md` finds nothing. Replaced by 7.2 after plan review round 3.
- [x] 3.2 Add archive guidance to `openspec/config.yaml`: before archiving, check the pull request for a **Code review** section, and run `peer-code-review` first when it is missing. Verify: `openspec instructions archive --change peer-review-skills --json` lists the entry under `operationGuidance`. Replaced by 7.3 after plan review round 3.

## 4. Plan review in the schema

- [x] 4.1 In the `review` instruction in `openspec/schemas/spec-driven-review/schema.yaml`, replace the lines that call `critique` and map its output. Add the four-step method, the step that picks the reviewer through `choose-an-adversary`, the fresh-context subagent fallback, and the rule that every blocking finding is Critical. Drop the stale `test-plan` mentions. Change no other verdict rule. Verify: `git grep -n -E "critique|test-plan" -- openspec/schemas/spec-driven-review/schema.yaml` finds nothing (a comment in `templates/review.md` still names `test-plan`, and stays, because the template must not change), `git diff main -- openspec/schemas/spec-driven-review/templates/review.md` is empty, and the diff of `schema.yaml` touches only the `review` instruction. Replaced by 7.4 after plan review round 3.

## 5. Remove `critique`

- [x] 5.1 Delete `.agents/skills/critique/`, and name `peer-code-review` instead of `critique` as the example of an authored skill in `CONTRIBUTING.md`. Verify: `git grep -n critique -- ':!openspec/changes' ':!openspec/specs/dev-environment'` finds nothing. The archive step updates the `dev-environment` spec.
- [x] 5.2 Check the `dev-environment` delta scenarios. Verify: after `just skills`, `git status --porcelain` prints nothing, `.claude/skills/peer-code-review/SKILL.md` exists, and a second `just skills` changes no file under `.agents/skills`.

## 6. Final checks

- [x] 6.1 Validate the change. Verify: `openspec validate peer-review-skills --strict` passes.

## 7. One skill per review (plan review round 3)

- [x] 7.1 Write `.agents/skills/peer-plan-review/SKILL.md` as design.md describes. It holds the author lookup (provenance note, then this session, then ask), the reviewer table with `codex exec -s read-only`, `claude -p --permission-mode plan`, and `agy --mode plan`, the fallbacks, the four-step method, the plain-language check, the injection rule, and the Critical, Moderate, and Suggestion labels. Its description triggers on a request to review a plan, design, spec, ADR, or skill, or on the `review` artifact, and not on writing one. Verify: the skill names all three commands with their read-only settings, and its description contains neither "written" nor "revised".
- [x] 7.2 Rewrite `.agents/skills/peer-code-review/SKILL.md` with its own reviewer table and fallbacks, as design.md describes. It finds the author from the `Co-Authored-By` lines in `<base>..HEAD` and picks a family that wrote none of them. Its **Code review** section names the reviewed commit. Verify: `grep -n choose-an-adversary .agents/skills/peer-code-review/SKILL.md` finds nothing, and the skill names `Co-Authored-By` and the reviewed commit.
- [x] 7.3 Change the archive guidance in `openspec/config.yaml`: run `peer-code-review` first when the **Code review** section is missing, or when later commits change files outside `openspec/changes/`. Verify: `openspec instructions archive --change peer-review-skills --json` shows the new entry under `operationGuidance`.
- [x] 7.4 Point the `review` instruction in `openspec/schemas/spec-driven-review/schema.yaml` at `peer-plan-review`. Move the four-step method into the skill. Keep the OpenSpec-only checks, the rule that every blocking finding is Critical, and the verdict rules. In `templates/review.md`, drop `test-plan, ` from the one comment that names it. Verify: `git grep -n -E "choose-an-adversary|test-plan" -- openspec/schemas` finds nothing, and `git diff main -- openspec/schemas/spec-driven-review/templates/review.md` shows only that comment.
- [x] 7.5 Delete `.agents/skills/choose-an-adversary/`. Verify: `git grep -n -E "critique|choose-an-adversary" -- ':!openspec/changes' ':!openspec/specs/dev-environment'` finds nothing.
- [ ] 7.6 Re-check the `dev-environment` delta scenarios and validate. Verify: after `just skills`, `git status --porcelain` prints nothing, `.claude/skills/peer-plan-review/SKILL.md` exists, a second `just skills` changes no file under `.agents/skills`, and `openspec validate peer-review-skills --strict` passes.
