# Sidereal v2 web shell

The web shell is the user interface for Sidereal v2. It is built with Vite,
React 19, and TypeScript. Its one screen shows whether the v2 server in
`server/` is running: `checking`, `healthy`, or `unreachable`. It checks again
every 5 seconds.

The web shell has its own `package.json` and `pnpm-lock.yaml`. It does not use
the root npm workspace, so you do not need to run `npm install` at the repo
root.

## Before you start

You need Node 26 and pnpm 12.

- **With the Nix shell**, you already have both. See
  [`CONTRIBUTING.md`](../CONTRIBUTING.md#development-environment).
- **Without Nix**, install pnpm once:

  ```bash
  npm i -g pnpm@12
  ```

`package.json` pins one exact pnpm release in its `packageManager` field. Any
pnpm 12 you run in `web/` switches to that release. The first time, pnpm
downloads it, so **your first pnpm run in `web/` needs network access**. Later
runs use the cached copy.

## Run the web shell and the server

From the repo root:

```bash
just dev
```

This starts the v2 server and the web shell. Open <http://localhost:5173>. The
page shows `healthy` once the server has built and started. Press Ctrl-C to
stop both.

## Run the web shell on its own

From the repo root:

```bash
just web
```

This installs dependencies from `pnpm-lock.yaml`, then starts the dev server on
<http://localhost:5173>. Without the server, the page shows `unreachable`.

`just web` never changes `pnpm-lock.yaml`. If you add a dependency to
`package.json`, run `pnpm install` in `web/` to update the lockfile, and commit
both files.

## Port 5173 is shared with the v0.10.x frontend

The web shell and the v0.10.x frontend both use port 5173. Run one stack at a
time. If port 5173 is taken, the web shell stops with
`Port 5173 is already in use` instead of moving to another port. Stop the other
dev server, then try again.

## How the web shell reaches the server

The browser asks the web shell's own origin for `/healthz`. The Vite dev server
forwards that request to the v2 server at `http://localhost:5000`. The web
shell's dev server sends no CORS headers, and it listens on this machine only.
