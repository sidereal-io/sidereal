## 1. ADR-005 and the docs that repeat it

- [x] 1.1 Amend ADR-005 in place, as design.md D9 describes: an agent copies tokens into Penpot over the Penpot MCP server, Penpot may lag until then, and `tokens.css` is the only generated file. In the same commit, update `DESIGN.md` ("How to use this file" and "Changing any token"), the third web UI rule in `AGENTS.md`, and the `tasks` rule in `openspec/config.yaml`. Verify: `grep -n -i "json\|import" DESIGN.md AGENTS.md openspec/config.yaml docs/decisions/ADR-005-visual-design-system.md` finds no remaining mention of a token JSON file or a manual Penpot import.

## 2. Dependencies and setup

- [x] 2.1 Add `@google/design.md` (exact version) and `yaml` as dev dependencies, and `@fontsource-variable/inter` and `@fontsource-variable/atkinson-hyperlegible-mono` as dependencies, in `web/package.json`. Verify: `pnpm install --frozen-lockfile` passes in `web/` with the updated lockfile.
- [x] 2.2 Add `scripts/**/*.ts` to the `include` list in `web/tsconfig.node.json`. Verify: `pnpm typecheck` passes, and fails after a number is assigned to a `string` variable in a file under `web/scripts/`.

## 3. The `DESIGN.md` reader and the generator

- [x] 3.1 Write `web/scripts/design-md.ts`, which reads `DESIGN.md` and parses its front matter (D2), with unit tests. Verify: the tests pass, including one that fails clearly when the front matter is missing.
- [x] 3.2 Write the generator, `web/scripts/tokens.ts`, so that it writes `web/src/styles/tokens.css` as D3 describes, with unit tests written first. Verify: the tests show `--sr-obsidian-950: #0B0A0F`, `--sr-font-size-14: 14px`, `--sr-opacity-45: 0.45`, and one property per primitive.
- [x] 3.3 Add the generator's refusals (D4) with a unit test for each: a semantic token, a component token, a duplicate key, a bad key, a bad `css-prefix`, a value with a refused character, a value that does not fit its group's form, and an unknown `x-sidereal` group. Verify: each spec scenario under "The generator refuses tokens it cannot write yet" passes, and the error names the token.
- [x] 3.4 Add the check mode and the `tokens` and `tokens:check` scripts. Generate and commit `web/src/styles/tokens.css`, and list it in `web/.prettierignore`. Verify: `pnpm tokens:check` passes; after a hand edit to `tokens.css`, it fails, prints `pnpm tokens`, and leaves the file unchanged.

## 4. The `DESIGN.md` lint

- [x] 4.1 Write `web/scripts/design-lint.ts` and the `design:lint` script (D5). It fails on any error, and on any warning except the three known ones, matched by rule and path. Verify: `pnpm design:lint` passes on today's `DESIGN.md`; it fails with an `x-other` key, printing `x-other`; it fails with `obsidian-950` set to `"#GGGGGG"`.

## 5. The contrast test

- [x] 5.1 Write `web/scripts/contrast.test.ts` (D6). Verify: it passes on today's `DESIGN.md`, and fails for each failing scenario in the spec: `obsidian-500` set to `#6F6B7C`, a cell changed to `7.5`, a cell changed to `TBD`, a table with no rows, and `(fails)` removed from an `obsidian-600` cell.

## 6. Fonts and the health screen

- [x] 6.1 Write `web/src/styles/fonts.css` with the two `@font-face` rules (D7), and `web/scripts/fonts.test.ts`. Verify: the test passes, and fails when a file name in a `url()` is misspelled.
- [x] 6.2 Write `web/src/styles/base.css`, and import `tokens.css`, `fonts.css`, and `base.css` in `web/src/main.tsx` (D8). Verify in a browser on `just dev`: the computed body styles are `rgb(11, 10, 15)`, `rgb(245, 244, 239)`, and a font family that starts with `Inter`; after `await document.fonts.ready`, Inter shows as loaded; the network log shows no request to another origin.
- [x] 6.3 Write `web/scripts/tokens-in-use.test.ts` (D8). Verify: it passes; it fails, naming `--sr-obsidian-950`, after `css-prefix` changes to `foo` and the generator runs; it fails, naming `--sr-obsidian-100`, after `obsidian-100` is removed and the generator runs.

## 7. The gate and CI

- [x] 7.1 Add `design:lint` and `tokens:check` to the `check-web` recipe in `justfile`. Verify: `just check-web` exits with status 0 on a clean checkout, and `git status --porcelain` prints nothing afterwards.
- [ ] 7.2 Add `DESIGN.md` to both path filters in `.github/workflows/ci.yml` (D10). Verify: the pull request's `server` and `web` jobs both run and pass.

## 8. Docs

- [x] 8.1 Explain in `web/README.md` how to change a token: edit `DESIGN.md`, run `pnpm tokens`, then run `just check-web`. Verify: the steps work as written on a scratch token change, which is then reverted.

## 9. Final check

- [x] 9.1 Run `just check`, and walk the failing scenarios in the `design-tokens`, `dev-commands`, and `web-shell` specs that the earlier tasks did not cover. Verify: `just check` exits with status 0, and each walked scenario behaves as the spec says.
