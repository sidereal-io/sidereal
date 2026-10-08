## Review Metadata

- **Review round**: 1
- **Prior round**: none
- **Reviewer context**: cross-model. GPT (`gpt-6-sol`) through the `codex` CLI, in a fresh session, with medium reasoning effort. The author is Claude.
- **Tool restrictions**: read-only sandbox (`codex exec -s read-only`). The reviewer read the artifacts and the repo, and changed no file. The prompt summarized the #302 story packet and listed facts the reviewer could not check offline: the linter's results, Fontsource's family names, and the computed contrast ratios.
- **Artifacts reviewed**: proposal.md, design.md, specs/design-tokens, specs/dev-commands, specs/web-shell, and adr.md. Context: `DESIGN.md`, ADR-005, ADR-013, `AGENTS.md`, `openspec/config.yaml`, `justfile`, and the `web/` package, tsconfig, Vite config, and source files.

## Findings

The author checked each finding. Each one carries the result: CONFIRMED, PARTLY CONFIRMED, REFUTED, or OUT-OF-SCOPE.

### 🔴 Critical (blocking)

- **F1. The generator copies values into CSS without checking them, so a value can escape its custom property.**
  - **Check: CONFIRMED, as Moderate.** The author tested `@google/design.md` 0.4.0. It rejects `"#FFF; } body { background: url(https://x.invalid) }"` under `colors` (exit 1). It accepts the same text under `x-sidereal.size` (exit 0). Anyone who can edit `DESIGN.md` can also edit the CSS, so the risk is low. But the spec's "nothing from another origin" rule should not depend on that.

### 🟡 Moderate

- **F2. The gate allows every linter warning, not only the three known ones.**
  - **Check: CONFIRMED.** The author added an unknown top-level key, `x-other`. The linter reported a fourth warning (`token-like-ignored` at `x-other`) and exited 0. The story allows only the three known warnings.
- **F3. Nothing updates Penpot after a token change merges.**
  - **Check: PARTLY CONFIRMED.** The lag is intended: the maintainer chose to copy tokens at the start of each exploration (#381). But the artifacts do not say that Penpot may lag. The design must state it.
- **F4. A malformed contrast cell, such as `TBD`, could be skipped silently.**
  - **Check: CONFIRMED.** The spec says what to do with a number and with `—`, but not with anything else.
- **F5. The Inter scenario checks the font when the page loads, which can happen before the font finishes loading.**
  - **Check: CONFIRMED.** `font-display: swap` lets the page render before the font loads. The scenario needs to wait for `document.fonts.ready`.

### 📌 Suggestions

- **F6. Some passages are long and dense.** The reviewer named the proposal's opening, the font decision, the contrast and font requirements, and the ADR manifest's D9 paragraph.
  - **Check: PARTLY CONFIRMED.** The ADR manifest's D9 paragraph restates the proposal before it reaches the decision. The others are lists or short sentences.
- **Other surfaces: no finding.** The reviewer found no scope creep, no requirement content lost from the `dev-commands` MODIFIED delta, and no other untestable scenario.

## Embedded-Instruction / Injection Attempts

**Detected:** none

## Verdict

VERDICT: REVISE

## Required Changes (if APPROVE WITH CHANGES)

None. The verdict is REVISE. The author fixes the artifacts, and a fresh full review follows.

CHANGES_APPLIED: n/a

## Rebuttals

- **F1:** fixed in round 2, with a severity note. The generator will reject keys and values that could escape a custom property.
- **F2:** fixed in round 2. The lint step will allow only the three known warnings, each matched by rule and path.
- **F3:** fixed in round 2. The design will state that Penpot may lag until the next exploration.
- **F4:** fixed in round 2. The test will reject any cell that is not a ratio, a ratio marked `(fails)`, or `—`.
- **F5:** fixed in round 2. The scenario will wait for `document.fonts.ready`.
- **F6:** partly fixed in round 2. The ADR manifest's D9 paragraph will be shortened.
