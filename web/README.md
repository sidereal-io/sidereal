# Sidereal web

The browser interface for Sidereal, built with Vite, React, and TypeScript.

It has its own `package.json` and `pnpm-lock.yaml`, and the repo root has no npm
project, so you never run `npm install` at the repo root.

## Before you start

You need Node and pnpm. The Nix shell provides both. Without Nix, see
[`CONTRIBUTING.md`](../CONTRIBUTING.md#development-environment).

`package.json` pins one exact pnpm release in its `packageManager` field. The
pnpm you run in `web/` switches to that release. The first time, pnpm downloads
it, so **your first pnpm run in `web/` needs network access**. Later runs use
the cached copy.

## Run it

From the repo root:

```bash
just web
```

This installs dependencies from `pnpm-lock.yaml`, then starts the dev server on
<http://localhost:5173>.

`just web` never changes `pnpm-lock.yaml`. If you add a dependency to
`package.json`, run `pnpm install` in `web/` to update the lockfile, and commit
both files.

## Check it

From the repo root:

```bash
just check-web
```

This installs dependencies from `pnpm-lock.yaml`, then runs six checks in
order. It stops at the first one that fails. `just check` runs it too, after
the server checks.

To run one check alone, run its script in `web/`:

| Command             | What it checks                                                                  |
| ------------------- | ------------------------------------------------------------------------------- |
| `pnpm typecheck`    | Types in every TypeScript file, including config, scripts, and tests.           |
| `pnpm lint`         | ESLint rules for TypeScript, React, and hooks.                                  |
| `pnpm format:check` | That every file matches Prettier's output.                                      |
| `pnpm design:lint`  | `DESIGN.md`, with the DESIGN.md linter. Only its three known warnings may pass. |
| `pnpm tokens:check` | That `src/styles/tokens.css` matches `DESIGN.md`.                               |
| `pnpm test`         | The unit tests, once, including the check of `DESIGN.md`'s contrast table.      |

To fix formatting, run `pnpm format`. To rerun the tests on every save, run
`pnpm vitest`.

## Change a design token

[`DESIGN.md`](../DESIGN.md) holds every token value. `src/styles/tokens.css` is
generated from it, so never edit that file by hand.

1. Edit the token in `DESIGN.md`.
2. In `web/`, run `pnpm tokens` to regenerate `src/styles/tokens.css`.
3. Run `just check-web` from the repo root.
4. Commit `DESIGN.md` and `src/styles/tokens.css` together.

If `pnpm tokens` refuses a token, its message names the token and says what is
wrong. Semantic and component tokens are refused until the generator can
resolve references.

When you change a color, update its rows in the table under `### Contrast` in
`DESIGN.md`. The contrast test checks every ratio in that table.

Stylesheets use tokens through `var()`, such as `var(--sr-obsidian-950)`. A
test fails if a stylesheet uses a custom property that no stylesheet defines.

## Port 5173 is shared with the v0.10.x frontend

This app and the v0.10.x frontend both use port 5173. Run one at a time. If
port 5173 is taken, the dev server stops with `Port 5173 is already in use`
instead of moving to another port. Stop the other dev server, then try again.

## How requests reach the server

The browser sends API requests, such as `/healthz`, to this app's own origin.
The Vite dev server forwards them to `http://localhost:5000`. The dev server
sends no CORS headers, and it listens on this machine only.
