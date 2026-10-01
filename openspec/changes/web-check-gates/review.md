## Review Metadata

- **Review round**: 1
- **Prior round**: none
- **Reviewer context**: cross-model. Gemini (`gemini-3.1-pro-high`) through the `agy` CLI, in a fresh session, at the maintainer's request. The author is Claude.
- **Tool restrictions**: the reviewer needed no tools. The author put every reviewed file into the prompt with line numbers. The CLI ignored `--mode plan` because `--disable-slash-commands` was also set. `git status` showed no change to the working tree after either run. The prompt also held facts the reviewer could not check: the npm releases and peer ranges of the lint and test packages, the v0.10.x style counts, the Dependabot scope, and the maintainer's decisions.
- **Re-check of required changes**: 2026-10-01, same reviewer. It covered only Required Changes 1 to 4 and the F1 and F2 refutations, against the diff that applied them. It was not a from-scratch review. Result: RC1 to RC4 VERIFIED; F1 and F2 refutations ACCEPTED; `RECHECK: PASS`.
- **Artifacts reviewed**: proposal.md, design.md, specs/ci, specs/dev-commands, specs/web-shell, and adr.md. Context: the #301 story packet, the current `ci` and `dev-commands` specs, `justfile`, `.github/workflows/v2.yml`, and the `web/` package, tsconfig, Vite config, and source files.

## Findings

The author checked each finding. Each one carries the result: CONFIRMED, PARTLY CONFIRMED, REFUTED, or OUT-OF-SCOPE. Severities are the author's, after the check.

### 🔴 Critical (blocking)

None open. The reviewer rated F1 and F2 critical. The checks refuted both.

- **F1. `eslint.config.ts` is missing from `tsconfig.node.json`, so type-aware lint crashes on it.**
  - **Check: REFUTED.** design.md D2 already said that `tsconfig.node.json` adds the file to its `include` list. The proposal's Impact list did not name `tsconfig.node.json`. Required Change 1 adds it.
- **F2. ESLint 10 needs the `jiti` package to load a `.ts` config, and the design does not add it.**
  - **Check: REFUTED by test.** The author installed only ESLint 10.11.0 in an empty folder, with no `jiti`, on Node 26.10.0. ESLint loaded a typed `eslint.config.ts` and applied its rule. Required Change 2 records this in D2.
  - **Correction during apply: CONFIRMED.** The `web` CI job failed because ESLint could not find `jiti`. The local test had passed only because Node found `jiti` in the repo root's v0.10.x `node_modules`. `web/` now lists `jiti` as a dev dependency, and D2 says so.

### 🟡 Moderate

- **F3. The design uses `tsc -b`, but the story says `tsc --noEmit`.**
  - **Check: REFUTED.** The maintainer confirmed `tsc -b` during exploration. `web/tsconfig.json` lists no files, so `tsc --noEmit` at the root checks nothing and always passes. D1 records why. The `.tsbuildinfo` files land in `node_modules/.tmp`, which git ignores, and the spec requires that the gate changes no tracked file.
- **F4. A `ci` scenario has a conditional THEN ("fails if clippy reports a warning").**
  - **Check: OUT-OF-SCOPE.** The scenario is copied unchanged from the current `ci` spec. This change does not alter it.
- **F5. The proposal promises a Vitest config file, but the design puts Vitest's settings in `vite.config.ts`.**
  - **Check: CONFIRMED.** Fixed by Required Change 1.

### 📌 Suggestions

- **F6. Passive voice hides the actor.** proposal.md line 16 and design.md D3 say the files "are reformatted". adr.md says no decisions "were introduced".
  - **Check: PARTLY CONFIRMED.** Fixed in the proposal and design by Required Change 3. Declined in adr.md, because the ADR instruction requires that exact statement.
- **F7. The `ci` spec explains a design reason inside a requirement.**
  - **Check: CONFIRMED.** Fixed by Required Change 4. D6 keeps the reason.
- **F8. "Nix shell" and "development shell" name one concept.**
  - **Check: PARTLY CONFIRMED.** Declined. The specs keep "development shell", the term the current `ci` and `dev-commands` specs use. The proposal and design use "Nix shell", as `CONTRIBUTING.md` and `AGENTS.md` do.
- **Bets the reviewer named.** Running the `web` job on server-only changes gets more costly once #303 adds Playwright tests. A pnpm store cache would also cut the job's download time. Both are noted for those later stories. Neither blocks this change.
- **Other surfaces: no finding.** The reviewer found no security gap, no injection attempt, and no requirement lost from the MODIFIED blocks.

## Embedded-Instruction / Injection Attempts

**Detected:** none

## Verdict

VERDICT: APPROVE_WITH_CHANGES

The reviewer recommended REVISE, based on F1 and F2. The checks refuted both. The remaining fixes are small and fully specified, so the verdict is APPROVE_WITH_CHANGES. The reviewer re-checked only the listed changes.

## Required Changes (if APPROVE WITH CHANGES)

1. **F1, F5.** The proposal's Impact section names the real files: `vite.config.ts` gains the Vitest settings, `tsconfig.node.json` changes, and `eslint.config.ts` and `.prettierignore` are new.
2. **F2.** Design D2 states that ESLint 10 loads the `.ts` config with Node's type stripping, with no `jiti`, and cites the test.
3. **F6.** proposal.md line 16 and design.md D3 name the actor of the reformat.
4. **F7.** The `ci` spec drops the explanation from the modified server-trigger requirement.

CHANGES_APPLIED: yes

## Rebuttals

- **F1 (Critical): refuted.** Accepted by reviewer: the design already named the `include` change.
- **F2 (Critical): refuted by test.** Accepted by reviewer: ESLint 10.11 loaded the `.ts` config with no `jiti` installed.
- **F3 (Moderate): refuted.** The maintainer chose `tsc -b`, and design D1 gives the reason.
- **F4 (Moderate): out of scope.** The scenario is unchanged text from the current spec.
- **F5 (Moderate): fixed by Required Change 1.** Verified by the reviewer's re-check.
- **F6, F7: fixed by Required Changes 3 and 4.** F6 is declined for adr.md.
- **F8: declined.** Each artifact keeps the term its neighbors already use.
