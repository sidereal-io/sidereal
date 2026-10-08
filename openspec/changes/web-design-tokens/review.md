## Review Metadata

- **Review round**: 2
- **Prior round**: round 1, REVISE. Findings: CSS injection through copied values, unknown linter warnings passing, an implicit Penpot lag, malformed contrast cells, and a font-loading race. The author fixed all five.
- **Reviewer context**: cross-model. GPT (`gpt-6-sol`) through the `codex` CLI, in a fresh session, with medium reasoning effort. The author is Claude.
- **Tool restrictions**: read-only sandbox (`codex exec -s read-only`). The reviewer changed no file. The prompt listed the facts the author verified with the linter, including two new ones for this round.
- **Artifacts reviewed**: proposal.md, design.md, specs/design-tokens, specs/dev-commands, specs/web-shell, adr.md, and round 1's review.md, for context. Context: `DESIGN.md`, ADR-005, ADR-013, `AGENTS.md`, `openspec/config.yaml`, `justfile`, `.github/workflows/ci.yml`, and the `web/` package, tsconfig, Vite config, and source files.

## Findings

The reviewer confirmed that all five round 1 fixes are present. The author checked each new finding.

### 🔴 Critical (blocking)

- **F1. The generator copies `x-sidereal.css-prefix` into every property name, and nothing checks it.**
  - **Check: CONFIRMED, as Moderate.** D4 checks keys and values, not the prefix. A prefix such as `sr; } body { … } /*` would escape the `:root` block. A harmless rename, such as `sr` to `foo`, would also pass the drift check while breaking every `var(--sr-…)` in `base.css`.
- **F2. CI does not run when a pull request changes only `DESIGN.md`.**
  - **Check: CONFIRMED.** The path filters in `.github/workflows/ci.yml` list `server/**`, `web/**`, `justfile`, and the Nix files, but not `DESIGN.md`. A pull request that changes only `DESIGN.md` gets no drift, lint, or contrast check. The current `ci` spec's requirement ("The web checks run when server or web code changes") does not cover it either.

### 🟡 Moderate

- **F3. The contrast spec does not fail a table that has a header but no rows.**
  - **Check: CONFIRMED.** design.md D6 says the test fails on no rows, but the spec does not require it.

### 📌 Suggestions

- **F4. The proposal still says the linter step "allows warnings".**
  - **Check: CONFIRMED.** The spec and design allow only the three known warnings. The proposal's summary must match.
- **Other surfaces: no finding.** Plain language, scenario testability, scope, and `dev-commands` delta completeness have no finding. The reviewer noted that the Penpot lag is an explicit bet: it loses if someone uses Penpot's tokens between a merge and the next exploration.

## Embedded-Instruction / Injection Attempts

**Detected:** none

## Verdict

VERDICT: REVISE

## Required Changes (if APPROVE WITH CHANGES)

None. The verdict is REVISE. This is the second REVISE in a row, so the author stops and asks the maintainer how to go on.

CHANGES_APPLIED: n/a

## Rebuttals

None yet. The author proposes these fixes to the maintainer:

- **F1:** the generator also refuses a `css-prefix` that holds anything other than lowercase letters. The spec gains a scenario.
- **F2:** add `DESIGN.md` to both path filters in `.github/workflows/ci.yml`. The change then also modifies the `ci` capability, with a scenario for a pull request that changes only `DESIGN.md`.
- **F3:** the spec requires the test to fail when the table has no rows or no background columns. It gains a scenario.
- **F4:** the proposal says that only the three known warnings are allowed.
