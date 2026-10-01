## Why

The v2 rewrite has a Rust server but no v2 user interface. ADR-005 calls for a new web shell whose first screen does one real thing. Showing the server's live health is the smallest real thing it can do. This walking skeleton is the first path through both v2 stacks. It also gives #282, the first asset screen, a place to live.

## What Changes

- A new web shell in `web/`, built with Vite, React 19, and TypeScript. pnpm installs its dependencies. The web shell has its own `package.json` and `pnpm-lock.yaml`, and stays out of the root npm workspace.
- `web/package.json` pins one exact pnpm 12 release in `packageManager`. Any pnpm 12 switches itself to that release, and the committed lockfile holds its hashes.
- The web shell has one screen. It checks the server's `GET /healthz` through the Vite dev server's proxy, every 5 seconds. It shows one of three states: checking, healthy, or unreachable.
- **BREAKING (contributors only):** `just dev` now runs the v2 server and the v2 web shell. It used to run the v2 server with the v0.10.x frontend, which cannot talk to that server.
- `just dev` starts both halves from a short shell recipe, and stops both when either one exits. It no longer needs `npx concurrently` from the v0.10.x npm tree.
- A new `just web` recipe installs the web shell's dependencies from the lockfile, then serves it.
- **BREAKING (contributors only):** `just frontend` becomes `just v0-frontend`. A new `just v0-dev` runs the whole v0.10.x stack through `npm run dev`.
- `just --list` groups the recipes for each stack under `v2` and `v0.10.x`.
- The Nix shell provides pnpm 12. The flake check reports `pnpm --version`.
- `CONTRIBUTING.md` gives the one-time pnpm step for contributors without Nix. A new `web/README.md` explains how to run the web shell.
- The web shell keeps Vite's default port, 5173. The v0.10.x frontend uses the same port. A contributor runs one stack at a time. If both run, the web shell stops with an error instead of moving to another port. This replaces the issue's note that `web/` needs a separate port.

## Capabilities

### New Capabilities

- `web-shell`: the v2 web shell in `web/`. It covers how the web shell installs and runs, its pinned pnpm release, and its server health screen.
- `dev-commands`: the root `justfile` recipes that run each stack for development.

### Modified Capabilities

- `dev-environment`: the shell's pinned toolchain gains pnpm 12, and the flake check reports its version.

## Impact

- **New code:** `web/`, including `web/README.md`.
- **Changed files:** `justfile`, `nix/toolchains.nix`, `.github/workflows/nix.yml`, `CONTRIBUTING.md`, `server/README.md`, and `AGENTS.md`.
- **v0.10.x:** no code changes. Only its `just` recipe names change.
- **Contributors:** anyone without Nix installs pnpm 12 once. The first pnpm run in `web/` downloads the pinned release, so it needs network access.
- **CI:** no workflow gains a web job here. CodeQL already scans every pull request, including `web/`. Lint, format, test, and CI gates for `web/` come in #301.
- **Open work:** #302 adds Tailwind and shadcn to `web/`. #303 adds a Playwright test of this screen.
