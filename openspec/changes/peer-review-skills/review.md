## Review Metadata

- **Review round**: 2, a scoped re-check of one plan edit, at the maintainer's choice. It is not a full review.
- **Prior round**: round 1, APPROVE_WITH_CHANGES. RC1 to RC5 were applied, and the re-check passed.
- **Reviewer context**: cross-model. GPT (`gpt-6-sol`) through the `codex` CLI, in a fresh session, with medium reasoning effort. The author is Claude. The prompt used `critique`'s method and the `review` instruction's checks.
- **Tool restrictions**: read-only sandbox (`codex exec -s read-only`). The reviewer changed no file.
- **Re-check of required changes**: 2026-10-09, same reviewer, one pass. It covered only RC1 to RC5, the F5 rebuttal, and defects the edits could add; it was not a from-scratch review. RC1 to RC5 VERIFIED, F5 rebuttal ACCEPTED, and one new Suggestion: adr.md still described code review as one guidance entry. The author fixed it. `RECHECK: PASS`.
- **Round 2 re-check**: 2026-10-09, same reviewer, one pass, scoped to RC6 and defects the edit could add. RC6 VERIFIED, including the skill; no new defect. `RECHECK: PASS`.
- **Artifacts reviewed**: proposal.md, design.md, adr.md, specs/dev-environment. Context: `.agents/skills/critique`, `.agents/skills/choose-an-adversary`, `.agents/skills/peer-code-review` and its rules file, `openspec/schemas/spec-driven-review/schema.yaml` and `templates/review.md`, `openspec/config.yaml`, `AGENTS.md`, `CONTRIBUTING.md`, and the current `dev-environment` spec.

## Findings

The reviewer recommended REVISE. It found no sentence over 30 words, no untestable scenario, and no reference to `critique` that the plan misses. The author checked each finding against the repo.

### 🔴 Critical (blocking)

- **F1. Code review rests on advisory guidance, so an agent can skip it.**
  - **Check: CONFIRMED.** The apply skill treats `operationGuidance` as "optional additive advice", and its done state suggests archive. Neither verify nor archive checks for a code review. The proposal says every change gets one.
- **F2. The `codex review` command sets no read-only sandbox.**
  - **Check: CONFIRMED.** The command in `peer-code-review` has no `-s read-only`, so it runs with whatever the user's Codex config allows. `codex -s read-only review` is accepted. The Claude and `agy` commands already run in plan mode.

### 🟡 Moderate

- **F3. The small-review rule conflicts with the `review` artifact's independence rule.**
  - **Check: CONFIRMED.** The design lets a small review run in the same session. The `review` instruction says the artifact MUST NOT be written in the context that wrote the plan. The template offers only "cross-model" or "fresh-context subagent".
- **F4. Dropping BLOCKS and NOTED leaves "blocking" undefined.**
  - **Check: CONFIRMED.** Today a BLOCKS finding maps to REVISE, whatever its severity. Once the reviewer reports Critical, Moderate, or Suggestion directly, nothing says which of them blocks.
- **F7 (round 2). The small-review rule lets a code review skip the cross-model reviewer.**
  - **Source:** the code review after apply (`codex review`), on `choose-an-adversary/SKILL.md`.
  - **Check: CONFIRMED.** The rule let `peer-code-review` run in the author's session for a small change while another family was installed. Its purpose, sparing small edits outside OpenSpec, went away with `critique`'s triggers. The design and proposal carried the rule, so the plan changes too.
- **F5. The delta spec checks none of the new review behavior.**
  - **Check: REFUTED.** No spec covers the review workflow today, including the existing `review` artifact. Both reviews are prompt-level instructions that agents follow, and no test can assert that. A new capability spec for them goes beyond the proposal. RC1 narrows the proposal's claim instead.

### 📌 Suggestions

- **F6. design.md points back to the proposal, and adr.md restates the four decisions.**
  - **Check: declined.** The `design` instruction asks for the pointer to the proposal. The manifest names each decision in one line to say why it needs no ADR, as earlier manifests do.

## Embedded-Instruction / Injection Attempts

**Detected:** none

## Verdict

VERDICT: APPROVE_WITH_CHANGES

## Required Changes (if APPROVE WITH CHANGES)

1. **RC1 (F1):** Say in the proposal and the design that both reviews are agent-honored, as the `review` artifact already is. Add a second check: archive guidance in `openspec/config.yaml` that tells the agent to look for the **Code review** section in the pull request before archiving, and to run `peer-code-review` first when it is missing.
2. **RC2 (F2):** Add `-s read-only` to the `codex review` command. In the design's table, give the read-only setting for each tool: `codex -s read-only`, `claude --permission-mode plan`, and `agy --mode plan`.
3. **RC3 (F3):** State in the design that the small-review rule never applies to the `review` artifact. When no other family is installed, the artifact is written by a fresh-context subagent, and its metadata says so.
4. **RC4 (F4):** State in the design that the reviewer marks every blocking finding Critical, as the template's "Critical (blocking)" heading already says.
5. **RC5 (maintainer request):** Move the code review rules to `REVIEW.md` at the repo root, and add one line to `AGENTS.md` that tells reviewers to apply it. Drop the `rules/` folder and the rules flags from `peer-code-review`. Record in the design the throwaway-repo test that chose this route. Update the matching line in adr.md.
6. **RC6 (F7):** Drop the small-review rule from proposal.md and design.md. A review runs in-session only when no other family is installed, and the `review` artifact then uses a fresh-context subagent. Record the dropped rule as an alternative in the design.

CHANGES_APPLIED: yes

## Rebuttals

- **F1:** fixed by RC1.
- **F2:** fixed by RC2.
- **F3:** fixed by RC3.
- **F4:** fixed by RC4.
- **F5:** rebutted, as described above. Accepted by reviewer: the proposal now says agents follow the reviews, and no tool enforces them.
- **F6:** declined (suggestion), as described above.
- **F7:** fixed by RC6.
- **Re-check suggestion (adr.md described code review as one guidance entry):** fixed. adr.md now names the apply and archive entries.
