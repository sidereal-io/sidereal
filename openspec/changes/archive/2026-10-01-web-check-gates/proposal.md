## Why

The v2 web shell in `web/` has no gate. Neither `just check` nor CI catches a type, lint, format, or test error in it. #282 is about to add the first real UI code. With the gates in place first, every line of that code is checked from the start. Mo, the maintainer, trusts one command and CI to gate both v2 stacks. A local gate that CI does not enforce drifts.

## What Changes

- `web/` gains four checks, each with its own `pnpm` script:
  - a type check of every TypeScript file in `web/`;
  - ESLint with a flat config, type-aware TypeScript rules, and React and hooks rules;
  - a Prettier check, with Prettier's default style;
  - Vitest with Testing Library, plus unit tests for the health screen.
- A new `just check-web` recipe installs `web/` from its lockfile, then runs the four checks. It fails when any check fails.
- **BREAKING (contributors only):** `just check` now runs `check-web` after `check-server`. Running it needs Node and pnpm, which the Nix shell already provides.
- The `v2` CI workflow gains a `web` job. The job runs `just check-web` inside the Nix shell, the same way the `server` job runs `just check-server`.
- The `v2` workflow also runs for changes under `web/`. Both jobs run for any change that triggers the workflow.
- This change reformats the existing files in `web/` to Prettier's default style. It also changes a few lines in `web/src/` so that they pass the new lint rules. The web shell's behavior stays the same.
- `web/README.md`, `server/README.md`, `CONTRIBUTING.md`, and `AGENTS.md` describe the new gate.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `ci`: the `v2` workflow runs for changes under `web/` and gains a `web` job that runs `check-web` inside the Nix shell. The local gate runs `check-web` too.
- `dev-commands`: a new `check-web` recipe, in the `v2` group, fails on a type, lint, format, or test error in `web/`.
- `web-shell`: the web shell has unit tests for its health screen, and they run without a server or a browser.

## Impact

- **Changed code:** `web/package.json`, `web/pnpm-lock.yaml`, the files in `web/src/`, `web/vite.config.ts` (which gains the Vitest settings), and `web/tsconfig.node.json`.
- **New files:** `web/eslint.config.ts`, `web/.prettierignore`, and the health screen's test file.
- **Changed files outside `web/`:** `justfile`, `.github/workflows/v2.yml`, `server/README.md`, `CONTRIBUTING.md`, and `AGENTS.md`.
- **Dependencies:** `web/` gains dev dependencies only: ESLint, its plugins, and `jiti` to load its TypeScript config; Prettier; and Vitest, Testing Library, and jsdom. The Nix shell needs no new tool.
- **Contributors:** `just check` takes a few seconds longer. Its first run in a fresh clone installs `web/` dependencies, which needs network access.
- **CI:** a change to `server/` now also runs the `web` job, and a change to `web/` also runs the `server` job. A change that touches neither, such as one to `apps/` only, runs neither.
- **v0.10.x:** no change. Its gate stays `npm run check`.
- **Out of scope:** Playwright tests (#303) and a pnpm store cache in CI.
