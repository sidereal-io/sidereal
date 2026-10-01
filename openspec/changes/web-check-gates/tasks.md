## 1. Format

- [x] 1.1 Add Prettier as a dev dependency of `web/`, a `web/.prettierignore` that lists `pnpm-lock.yaml`, and the `format` and `format:check` scripts (D3). Verify that `pnpm format:check` in `web/` fails on today's files.
- [x] 1.2 Run `pnpm format` in `web/`, and commit the result on its own with no other change (D3). Verify that `pnpm format:check` exits with status 0 and that `git diff --stat` for the commit touches only reformatted files.

## 2. Type check

- [x] 2.1 Add the `typecheck` script as `tsc -b` (D1). Verify that it exits with status 0. Then verify the `dev-commands` scenarios "A type error in app code" and "A type error in a config file" by running `pnpm typecheck` against a temporary edit, and undo each edit afterwards.

## 3. Lint

- [x] 3.1 Add ESLint 10, `@eslint/js`, typescript-eslint, `@eslint-react/eslint-plugin`, `eslint-plugin-react-hooks`, and `eslint-config-prettier` as dev dependencies. Write `web/eslint.config.ts` with the presets in D2's order, ignore `dist/` and `coverage/`, and add the `lint` script. Add `eslint.config.ts` to the `include` list in `web/tsconfig.node.json`. Verify that `pnpm lint` runs and reports the errors D7 expects, and that `pnpm typecheck` still exits with status 0.
- [x] 3.2 Fix every lint error in `web/src/` without changing behavior (D7): `void run()` in `HealthStatus.tsx`, and an explicit check for the `root` element in `main.tsx`. Verify that `pnpm lint` exits with status 0, and that the web shell still shows `healthy` with `just dev` running.
- [x] 3.3 Verify the `dev-commands` scenarios "A promise is left unhandled" and "A hook is called conditionally" by running `pnpm lint` against a temporary edit. Undo each edit afterwards.

## 4. Unit tests

- [x] 4.1 Add Vitest, `@testing-library/react`, `@testing-library/dom`, and jsdom as dev dependencies. Add the `test` key to `web/vite.config.ts`, with `defineConfig` imported from `vitest/config` and jsdom as the environment. Add the `test` script as `vitest run` (D4). Verify that `pnpm typecheck` still exits with status 0.
- [x] 4.2 Write `web/src/HealthStatus.test.tsx`: stub `fetch`, expect `healthy` for status 200 with `{"status":"ok"}`, and expect `unreachable` for status 500 (D4). Verify every scenario of the `web-shell` requirement "The health screen has unit tests", with nothing listening on port 5000. Break the component temporarily for the two failure scenarios, and undo each change afterwards.
- [x] 4.3 Verify that `pnpm lint` and `pnpm format:check` pass on the test file and on the changed `vite.config.ts`.

## 5. Recipes and CI

- [x] 5.1 Add the `check-web` recipe from D5, in group `v2`, and change `check` to `check: check-server check-web`. Update the `check` recipe's comment to name both gates. Verify every scenario of the `dev-commands` requirement "The web gate fails on any web check error". Run the "Dependencies are not installed yet" scenario after removing `web/node_modules`. Run the stale-lockfile scenario in a scratch clone under `.workspace/`.
- [x] 5.2 Verify the `dev-commands` scenario "A reviewer reads the recipe groups" and the `ci` scenario "The local gate runs the same recipe".
- [ ] 5.3 Add `web/**` to the `paths` filter in `.github/workflows/v2.yml`, and add the `web` job from D6, with the same action versions as the `server` job. Verify the `ci` scenarios "The web job has one check step" and "The web job has no step that installs a tool", and that the three `server` job scenarios still pass, with `yq` version 4.
- [ ] 5.4 Verify that the `web` and `server` jobs both pass on this change's pull request. That pull request changes `web/`, `justfile`, and the workflow file, so it triggers both jobs.

## 6. Docs

- [ ] 6.1 Update `web/README.md` with a "Check it" section: `just check-web`, each `pnpm` script, `pnpm format` to fix formatting, and `pnpm vitest` for watch mode. Verify that every command in it runs as written.
- [ ] 6.2 Update the recipe table in `server/README.md`, the gate lines in `AGENTS.md`, and the `server` job paragraph in `CONTRIBUTING.md`'s "Reviewing a pin update" section, so each names `check-web` and the `web` job. Verify that `git grep -n 'It runs \`just check-server\`'` finds no match outside `openspec/`.

## 7. Final check

- [ ] 7.1 Run `just check`, `nix flake check`, and `openspec validate web-check-gates --strict`. Verify that each exits with status 0 and that `git status --porcelain` prints nothing afterwards.
- [ ] 7.2 Walk every scenario in the three delta specs once more on the final commit. Record any that could not run, with the reason, in the PR description.
