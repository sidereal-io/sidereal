# Sidereal web

The browser interface for Sidereal, built with Vite, React, and TypeScript.

It has its own `package.json` and `pnpm-lock.yaml`, and the repo root has no npm
project, so you never run `npm install` at the repo root.

## Before you start

You need Node and pnpm. mise installs both. Without mise, see
[`CONTRIBUTING.md`](../CONTRIBUTING.md#without-mise).

`package.json` pins one exact pnpm release in its `packageManager` field. The
pnpm you run in `web/` switches to that release. The first time, pnpm downloads
it, so **your first pnpm run in `web/` needs network access**. Later runs use
the cached copy.

## Run it

From the repo root:

```bash
mise run web
```

Without mise, run `sh scripts/dev.sh web` instead.

This installs dependencies from `pnpm-lock.yaml`, then starts the dev server on
<http://localhost:5173>.

`mise run web` never changes `pnpm-lock.yaml`. If you add a dependency to
`package.json`, run `pnpm install` in `web/` to update the lockfile, and commit
both files.

## Check it

From the repo root:

```bash
mise run check-web
```

This installs dependencies from `pnpm-lock.yaml`, then runs six checks in
order. It stops at the first one that fails. `mise run check` runs it too, after
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

## End-to-end tests

`mise run e2e` runs the browser tests on a stack it makes for that run only,
and removes it afterwards. Start the database fixture first, with
`mise run db-up`. The runner, `scripts/e2e.sh`:

1. Refuses to run when anything already listens on its two ports. It derives
   them from the checkout, so they are never 5000 or 5173, and a running
   `mise run dev` is left alone.
2. Builds the server, and installs the web packages and Chromium.
3. Makes a throwaway database, `sidereal_test_e2e_<random>`, through the
   fixture's test role, and a run folder whose empty `storage/` folder is the
   server's `STORAGE_ROOT`. It ignores an inherited `DATABASE_URL` and
   `STORAGE_ROOT`.
4. Starts the server as its own child and waits for its listening line.
   Playwright then starts Vite, which forwards to that server through
   `SIDEREAL_DEV_PROXY_TARGET`.
5. On every exit, pass or fail, stops Playwright, Vite, Chromium and the
   server, removes the run folder and drops the database. If the tests pass
   but cleanup fails, it exits non-zero and names what it left behind.
   `mise run db-test-clean` removes a database a killed run left.

Arguments to `sh scripts/e2e.sh` go to `playwright test`. A
browser test can restart the server: it sends `SIGUSR1` to the process in
`SIDEREAL_E2E_RUNNER_PID`, then waits for `ready-<n>` in
`SIDEREAL_E2E_RUN_DIR`, where `<n>` counts the restarts.

Three checks cover the runner itself. Run them when you change it:

| Command                              | Checks                                                                 |
| ------------------------------------ | ---------------------------------------------------------------------- |
| `bash scripts/e2e-readiness.test.sh` | The wait for the server's listening line, with stub servers.           |
| `bash scripts/e2e-runner-check.sh`   | Restart, interrupt, a failing test and a busy port, on the real stack. |
| `sh scripts/dev-proxy-check.sh`      | That Vite forwards to the server `SIDEREAL_DEV_PROXY_TARGET` names.    |

The runner downloads the Chromium build that `@playwright/test` asks for the
first time it runs; after that the download is skipped, so there is no
separate install step.

Playwright owns its browser rather than using one your system provides, so
the versions cannot drift: bump `@playwright/test` in `package.json` and
Playwright fetches the matching browser. On NixOS that download cannot run,
because the browser expects a system dynamic loader. Point `PLAYWRIGHT_BROWSERS_PATH` at the
browsers from nixpkgs when their version matches the npm package, or run the
suite from a non-NixOS host or container.

## Change a design token

[`DESIGN.md`](../DESIGN.md) holds every token value. `src/styles/tokens.css` is
generated from it, so never edit that file by hand.

1. Edit the token in `DESIGN.md`.
2. In `web/`, run `pnpm tokens` to regenerate `src/styles/tokens.css`.
3. Run `mise run check-web` from the repo root.
4. Commit `DESIGN.md` and `src/styles/tokens.css` together.

If `pnpm tokens` refuses a token, its message names the token and says what is
wrong.

A semantic token, under `x-sidereal.semantic`, names one primitive. Its value
must be exactly `{<group>.<key>}`, such as `"{colors.violet-600}"` or
`"{font-size.font-size-14}"`, where the group is `colors`, `rounded`,
`spacing`, or a group under `x-sidereal`. The generator writes it as `var()` of
that primitive: `--sr-color-action-primary: var(--sr-violet-600);`. It still
refuses:

- a raw value, such as `"#7C3AED"`;
- a reference to a primitive that does not exist, or to another semantic token;
- any text before or after the reference, such as `"{colors.violet-600}; }"`;
- every component token, under `components`.

The Penpot export (`pnpm tokens:penpot`) carries primitives only, so it leaves
semantic tokens out.

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
