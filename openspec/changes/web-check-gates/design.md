## Context

See proposal.md for why. This section covers only the current state that shapes the approach.

- `web/tsconfig.json` has `"files": []` and two project references: `tsconfig.app.json` for `src/`, and `tsconfig.node.json` for `vite.config.ts`. Both child configs set `noEmit`.
- `web/` has no lint, format, or test tooling. The skeleton left the template's ESLint setup out on purpose, so this change could choose the rules.
- The v0.10.x tree has no formatter or linter either. Most of its files use semicolons and double quotes, which are Prettier's defaults. The three `web/src/` files use no semicolons and single quotes, from the Vite template.
- `web/` uses TypeScript 6.0.3, React 19, and Vite 8. pnpm 12 installs it from `web/pnpm-lock.yaml`.
- The `v2` workflow has one job, `server`, and one workflow-level `paths` filter. GitHub applies `paths` to the whole workflow, not to each job.
- On 2026-10-01, the latest releases were ESLint 10.11, typescript-eslint 8.71, eslint-plugin-react 7.37.5, eslint-plugin-react-hooks 7.1, @eslint-react/eslint-plugin 5.23, Prettier 3.9, Vitest 5.0, Testing Library for React 16.3, and jsdom 30.1.
- eslint-plugin-react 7.37.5 declares support for ESLint up to 9.x only. Its last release was in April 2025.
- typescript-eslint 8.71 declares support for TypeScript below 6.1.

## Goals / Non-Goals

**Goals:**

- Every TypeScript file in `web/` is type-checked, linted, and formatted, including config files and tests.
- A contributor runs the same web gate as CI, with one recipe.
- The rules are strict from the start, while `web/` is small enough that strictness costs little.

**Non-Goals:**

- Playwright and browser tests. #303 adds them.
- A pnpm store cache in CI.
- Dependabot updates for `web/`. The npm entry in `.github/dependabot.yml` covers the repo root only.
- Lint or format rules for the v0.10.x tree. Its gate stays `npm run check`.
- Accessibility lint rules. #282 can add them with its first real UI.

## Decisions

### D1. Type-check with `tsc -b`, not `tsc --noEmit`

The `typecheck` script runs `tsc -b`. Build mode follows the two project references, so it checks `src/` and `vite.config.ts` with their own settings.

- **Why not `tsc --noEmit`:** the root `tsconfig.json` lists no files, so `tsc --noEmit` at the root checks nothing and always passes.
- **Output:** both child configs set `noEmit`, so `tsc -b` writes only its `.tsbuildinfo` files, under `node_modules/.tmp`. Git ignores them.
- **Alternative:** run `tsc --noEmit -p` once for each child config. It gives the same result with two commands, and a third config would need a third command. Rejected.

### D2. Lint with ESLint 10, type-aware TypeScript rules, and React rules

`web/eslint.config.ts` is a flat config. It applies these presets, in this order:

1. `@eslint/js` `recommended`.
2. typescript-eslint `strictTypeChecked`, with `parserOptions.projectService` on. ESLint then reads types from the same tsconfig files that `tsc -b` uses.
3. `@eslint-react/eslint-plugin` `recommended-type-checked`, for React component rules.
4. `eslint-plugin-react-hooks` `recommended`, for the rules of hooks and the React Compiler checks.
5. `eslint-config-prettier`, last, which turns off every rule that formatting decides.

The config ignores `dist/` and `coverage/`. The `lint` script runs `eslint .` with ESLint's default settings, so any warning is reported but only an error fails the check.

- **Why type-aware rules:** they catch async mistakes, which polling UI code tends to make. `no-floating-promises` flags today's unhandled `run()` call in `HealthStatus.tsx`. `web/` is small, so the slower lint costs a second or two.
- **Why `@eslint-react` and not `eslint-plugin-react`:** eslint-plugin-react does not support ESLint 10. @eslint-react supports it, is written for TypeScript, and has a type-aware preset.
- **Why the config file is TypeScript:** `tsconfig.node.json` adds it to its `include` list. `tsc -b` then checks it, and ESLint's project service finds types for it. Every config file in `web/` is then type-checked. ESLint loads a `.ts` config through `jiti`, which becomes a dev dependency if ESLint 10 needs it.
- **Alternative: typescript-eslint `recommended` only.** It is faster and needs no tsconfig wiring, but it misses unhandled promises and unsafe use of `any`. Rejected.
- **Alternative: ESLint 9 with eslint-plugin-react.** It keeps the most common React plugin, but it starts a new app one major release behind. Rejected.

### D3. Format with Prettier's default style

Prettier runs with no options. `web/` has no Prettier config file. A `web/.prettierignore` lists `pnpm-lock.yaml`, because pnpm owns that file's format. Prettier already skips what `web/.gitignore` lists. The `format:check` script runs `prettier --check .`, and a `format` script runs `prettier --write .` for contributors.

- **Why the defaults:** most v0.10.x files already use them. Components ported from v0.10.x under ADR-005 then need no restyle.
- **The one-time reformat:** the existing files in `web/` are reformatted in their own commit, with no other change in it. The reformat is then easy to review and to skip in `git blame`.
- **Alternative: keep the template's style with `semi: false` and `singleQuote: true`.** It avoids the reformat, but `web/` would then differ from most v0.10.x code. Rejected.

### D4. Test with Vitest, jsdom, and Testing Library

Vitest reads its settings from the `test` key in `web/vite.config.ts`, which then imports `defineConfig` from `vitest/config`. Tests run in jsdom. The `test` script runs `vitest run`, which runs once and exits. A contributor runs `pnpm vitest` for watch mode.

Tests sit next to the code they test, such as `src/HealthStatus.test.tsx`. They import `describe`, `it`, `expect`, and `vi` from `vitest`. Vitest's globals stay off. Each test file calls Testing Library's `cleanup` after each test.

`HealthStatus.test.tsx` replaces `fetch` with `vi.stubGlobal`, renders the component, and waits for the state text with `findByText`. One test answers status 200 with `{"status":"ok"}` and expects `healthy`. Another answers status 500 and expects `unreachable`. Real timers run, because the first check starts at once and the component stops checking when the test unmounts it.

- **Why jsdom:** it is the most common DOM for Testing Library, and its behavior is well known.
- **Why explicit imports:** `tsconfig.app.json` covers the tests too. With explicit imports, the app's `types` list needs no test types, so app code cannot use test globals by mistake.
- **Why no `@testing-library/jest-dom`:** `findByText` fails when the text never appears, so these tests need no extra matchers. #282 can add jest-dom when its tests need richer checks.
- **Alternative: happy-dom.** It is faster, but it differs from browsers in more places. Rejected for now.

### D5. `check-web` installs, then runs the four checks in order

```just
# Web gate: install from the lockfile, then type check, lint, format check, tests.
[group('v2')]
check-web:
    cd web && pnpm install --frozen-lockfile --reporter=append-only && pnpm typecheck && pnpm lint && pnpm format:check && pnpm test
```

`check` becomes `check: check-server check-web`. `just` runs the two in that order, and stops at the first failure.

- **Why install first:** CI starts from a fresh clone, and a contributor's `node_modules` may be stale. pnpm finishes in about a second when nothing changed.
- **Why `--frozen-lockfile`:** a stale lockfile fails the gate instead of being rewritten, as in `just web`.
- **Why stop at the first failure:** `check-server` already stops at its first failure, and contributors expect the same.
- **Why the recipe lists the four scripts:** the order of checks is visible in one place, beside `check-server`.

### D6. Both `v2` jobs share one path filter

`.github/workflows/v2.yml` adds `web/**` to its `paths` filter. A new `web` job has three steps: `actions/checkout`, `DeterminateSystems/determinate-nix-action`, and `nix develop --command just check-web`. The job uses the same action versions as the `server` job.

- **Why one filter:** GitHub filters paths for the whole workflow. Both jobs therefore run for any change under `server/` or `web/`. Running a job that was not needed costs about a minute. Skipping a needed job could let a broken change merge.
- **Alternative: a path-detection action that skips each job.** Triggers would be exact, but it adds a third-party action and a job that only routes the others. Rejected.
- **Alternative: one workflow file per stack.** It contradicts the shared decision for this epic: v2 is one workflow with `server` and `web` jobs. Rejected.
- **No cache step:** the job downloads pnpm and the `web/` dependencies on every run. A pnpm store cache is out of scope.

### D7. Fix today's code to pass the new rules, with no change in behavior

The first lint run will report a few errors in `web/src/`. Each fix keeps the web shell's behavior the same. Two errors are already known:

- `HealthStatus.tsx` calls `run()` without handling its promise. `run()` never rejects, because `checkHealth` catches every error, so the call becomes `void run()`.
- `main.tsx` uses a non-null assertion on `document.getElementById('root')`. `strictTypeChecked` forbids it. The code checks for the element and throws an error that names the missing `root` element.

## Risks / Trade-offs

- **[Risk] typescript-eslint supports TypeScript below 6.1 only.** → `web/package.json` pins `typescript` with `~6.0.2`, so it stays at 6.0.x. A move to TypeScript 6.1 waits for a typescript-eslint release that supports it.
- **[Risk] Nothing updates the `web/` dependencies automatically.** → Dependabot covers the repo root only. Adding `web/` is separate work.
- **[Risk] React Compiler rules in eslint-plugin-react-hooks are new and may change between releases.** → The lockfile pins the release. A contributor who updates it runs `just check-web` and fixes any new error in the same pull request.
- **[Trade-off] `just check` now needs Node and pnpm.** → The Nix shell provides both. `CONTRIBUTING.md` already tells contributors without Nix to install pnpm 12.
- **[Trade-off] Both `v2` jobs run for any v2 change.** → It adds about a minute of CI time. Accepted, to keep one workflow with no routing job.
- **[Trade-off] The `web` job has no dependency cache.** → It downloads the `web/` dependencies on every run. Accepted until a cache story is needed.

## Migration Plan

- After pulling, contributors run `just check` as before. Its first run installs the new dev dependencies in `web/`.
- A contributor with an open `web/` branch rebases it, runs `pnpm format` in `web/`, and fixes any lint error.
- Rollback is a revert of the merge. No data or deployed system changes.
