# Sidereal web

The browser interface for Sidereal, built with Vite, React, and TypeScript.

It has its own `package.json` and `pnpm-lock.yaml`. It does not use the root
npm workspace, so you do not need to run `npm install` at the repo root.

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

## Port 5173 is shared with the v0.10.x frontend

This app and the v0.10.x frontend both use port 5173. Run one at a time. If
port 5173 is taken, the dev server stops with `Port 5173 is already in use`
instead of moving to another port. Stop the other dev server, then try again.

## How requests reach the server

The browser sends API requests, such as `/healthz`, to this app's own origin.
The Vite dev server forwards them to `http://localhost:5000`. The dev server
sends no CORS headers, and it listens on this machine only.
