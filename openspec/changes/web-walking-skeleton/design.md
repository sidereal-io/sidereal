## Context

See proposal.md for why. This section covers only the current state that shapes the approach.

- The v2 server serves `GET /healthz` and answers `{"status":"ok"}`. It listens on port 5000 unless `PORT` says otherwise.
- `just dev` runs `npx concurrently`. The `concurrently` package comes from the root `package.json`, which is the v0.10.x npm tree. On a clone with no root `npm install`, npx stops to ask before it downloads anything.
- Today's `just dev` pairs the v2 server with the v0.10.x frontend. That frontend calls `/api` and `/ws`, which the v2 server does not serve.
- The v0.10.x frontend uses Vite's default port, 5173. The v0.10.x server also defaults to port 5000.
- The root `tsconfig.json` includes only `apps/`, `packages/`, and `tools/scripts/`, so a new `web/` folder stays outside `npm run check`.
- The locked nixpkgs provides `pnpm_12` at 12.3.4. npm's newest pnpm 12 release is 12.8.2. Node 26 ships without corepack.
- The pinned `just` is 1.58, which supports the `[group]` recipe attribute.

## Goals / Non-Goals

**Goals:**

- A contributor runs the whole v2 stack with one command on a fresh clone, with no v0.10.x install.
- The first v2 screen does one real thing, and later screens can build on its layout.

**Non-Goals:**

- Choosing the web shell's data-fetching library or router. #282 makes those choices with its first data screen.
- Letting the dev proxy follow a `PORT` override on the server. The proxy targets port 5000 only.
- Serving a production build of `web/` from the v2 server.
- Running the v0.10.x and v2 stacks at the same time. They share ports 5000 and 5173.

## Decisions

### D1. Start `web/` from Vite's React and TypeScript template, trimmed

`web/` starts from the `react-ts` template of `create-vite`. The template's demo assets and styles go. `src/` keeps an entry file, an `App` component, and one health-status component.

- **Why:** the template is the standard layout for Vite with React. #301 to #303 add lint, Tailwind, and Playwright to it with their usual setup steps.
- **Lint config:** the template's ESLint file and packages stay out. #301 adds lint with its own rules.
- **Alternative:** copy the setup from `apps/client`. Rejected, because ADR-005 asks for a new shell, and that setup carries v0.10.x path aliases.

### D2. Pin pnpm 12 in `packageManager`, and let the lockfile verify it

`web/package.json` sets `packageManager` to an exact pnpm 12 release, such as `pnpm@12.8.2`, with no hash suffix. The Nix shell provides `pkgs.pnpm_12`. On first use, that pnpm downloads the pinned release and records it in `web/pnpm-lock.yaml`, under `packageManagerDependencies`. Git tracks that lockfile.

A test during exploration showed how pnpm 12 behaves:

| Test | Result |
|---|---|
| pnpm 12.3.4, with `packageManager: pnpm@12.8.2+sha512.<correct hash>` | Runs 12.8.2 |
| The same, with a wrong `+sha512` hash and an empty cache | Runs 12.8.2. pnpm 12 ignores the suffix. |
| A changed hash in the lockfile's pnpm entries | Refuses to run, with `ERR_PNPM_PNPM_ENGINE_IDENTITY_MISMATCH` |

- **Why no hash suffix:** pnpm 12 ignores it, so it would look like a check without being one.
- **Why `pnpm_12`, not `pnpm`:** the unversioned attribute moves to a new major when `flake.lock` moves. A major change could alter how pnpm reads the pin.
- **Contributors without Nix** run `npm i -g pnpm@12` once. Their pnpm then switches itself in the same way.
- **ADR-013 effect:** ADR-013 says every tool version comes from one committed lock file. For pnpm in `web/`, the release that runs comes from `web/package.json`, and `web/pnpm-lock.yaml` verifies it. The ADR step corrects that consequence in place.
- **Alternative: pnpm 10.** Its pin handling is well known, but it is two majors behind on a new app. Rejected.
- **Alternative: pin the exact release in Nix only.** nixpkgs lags npm, and contributors without Nix would have no pin. Rejected.
- **Alternative: corepack.** Node 26 ships without it. Rejected.

### D3. Run `just dev` as a short POSIX shell recipe that stops at the first exit

`just dev` starts `just server` and `just web` in the background. It waits until either one exits, then stops the other and exits with an error if either failed.

```just
[group('v2')]
dev:
    #!/bin/sh
    just server & s=$!
    just web & w=$!
    trap 'kill $s $w 2>/dev/null' INT TERM
    while kill -0 $s 2>/dev/null && kill -0 $w 2>/dev/null; do sleep 1; done
    kill $s $w 2>/dev/null
    wait $s; a=$?; wait $w; b=$?
    [ $a -eq 0 ] && [ $b -eq 0 ]
```

A test of this recipe with stand-in recipes, under Linux's `/bin/sh` (dash), showed:

- When `web` failed after 2 seconds, `just` printed `web`'s error, stopped `server`, and exited with status 1 within 3 seconds. No process was left running.
- One SIGINT, as Ctrl-C sends, stopped both recipes and left no process running.

- **Why:** it needs no npm package, so `just dev` no longer depends on the v0.10.x npm tree. A failure in either half stops the whole command, so the contributor sees it.
- **Why POSIX `sh`:** it runs with macOS's `/bin/sh`, so contributors on a Mac without Nix need no newer bash.
- **Output:** it has no per-process labels or colours. Cargo and Vite logs differ enough to tell apart.
- **Alternative: `just`'s `[parallel]` attribute.** It is one line, but `just` waits for every parallel recipe before it reports a failure. The server never ends, so a failed `web` would leave only one line of pnpm or Vite output, which cargo's build output can scroll away. Rejected after the round 1 review.
- **Alternative: `concurrently --kill-others-on-fail`, as a dev dependency of `web/`.** It keeps labelled output, but the web shell's manifest would then start the Rust server. Rejected.
- **Alternative: bash's `wait -n`.** It needs bash 4.3 or later, and macOS ships bash 3.2. Rejected.

### D4. `just web` installs from the lockfile, then serves

`just web` runs `pnpm install --frozen-lockfile`, then `pnpm dev`, both in `web/`.

- **Why install every time:** a fresh clone then works with one command. pnpm finishes in about a second when nothing changed.
- **Why `--frozen-lockfile`:** a stale lockfile stops the run instead of being rewritten. Only a deliberate `pnpm install` or `pnpm add` changes the lockfile.

### D5. Check health with a timed loop and plain `fetch`

One React component owns the check. It sends `GET /healthz` with `fetch`, and aborts it after 5 seconds. When a check finishes, it schedules the next one 5 seconds later. It stops when the component unmounts.

The component maps each result to one of three states, as the `web-shell` spec defines. It reads the body only after a 200 status. It parses the body as JSON and compares `status` with `"ok"`. Any error along the way maps to `unreachable`: a network error, an abort, or a failed parse. It never renders text from the body.

- **Why a timeout after each check:** checks never overlap, even when the server is slow to answer.
- **Why plain `fetch`, not a query library:** the data-fetching choice belongs to #282 and its first real data screen. One loop does not justify the dependency.
- **Why no body text on screen:** the page stays safe whatever the server, or something posing as it, sends back.

### D6. Forward `/healthz` through Vite's dev proxy

`web/vite.config.ts` forwards `/healthz` to `http://localhost:5000`. The browser only ever talks to the web shell's own origin.

- **Why:** same-origin requests need no CORS setup on the server. ADR-007 leaves the server's CORS and CSRF rules to later work.
- **Why only `/healthz`:** the server has no other routes yet. #282 adds the proxy entries for its API.
- **Proxy errors:** when the server is down, Vite's proxy answers with status 500 and logs a connection error. The screen shows `unreachable`. The log lines are expected while cargo builds.

### D7. Keep Vite's defaults for host and port

The web shell keeps Vite's default port, 5173, and its default host, `localhost`. `strictPort` is on.

- **Why the default port:** a contributor runs only one stack at a time, so the web shell and the v0.10.x frontend never compete for 5173.
- **Why `strictPort`:** without it, Vite moves to the next free port when 5173 is taken. A contributor who left the v0.10.x frontend running would open 5173 and see the wrong app. With `strictPort`, the web shell stops with an error that names the port.
- **Why the default host:** the dev server stays reachable only from this machine. The v0.10.x config listens on `0.0.0.0`. The web shell does not copy that.

### D8. Rename and group the recipes

- `frontend` becomes `v0-frontend`. A new `v0-dev` runs `npm run dev`, which starts the v0.10.x server, worker, and frontend.
- `[group('v2')]` marks `dev`, `server`, `web`, `check`, and `check-server`. `[group('v0.10.x')]` marks the two `v0-` recipes. `default`, `skills`, and `enter` stay ungrouped.
- **Why `v0-` and not `v0.10-`:** `just` recipe names cannot contain dots.
- **Why no `v0-server` or `v0-worker`:** the v0.10.x server needs its worker. `npm run dev:*` covers the rare case of running one part alone. The v0.10.x recipes go away at cutover, so they stay few.

### D9. Add pnpm to the shell and the flake check

`nix/toolchains.nix` adds `pkgs.pnpm_12`. The flake check's version step adds `pnpm --version`.

- **What the version shows:** at the repo root, `pnpm --version` reports the Nix release that starts pnpm, such as 12.3.4. In `web/`, pnpm reports the pinned release. The flake check runs at the root, so it reports the Nix release.

## Risks / Trade-offs

- **[Risk] The first pnpm run in `web/` needs network access** to download the pinned release. → pnpm caches the release, so later runs work offline. `web/README.md` says so.
- **[Risk] A contributor without Nix installs a different pnpm major.** → `CONTRIBUTING.md` names `pnpm@12`. The `web-shell` spec only promises that a pnpm 12 release switches itself.
- **[Risk] A later pnpm 12 release changes how it handles `packageManager`.** → `pnpm_12` moves only with `flake.lock`, and the flake check reports its version. A pull request that bumps it shows the change.
- **[Trade-off] `just dev` output has no labels.** → Accepted to drop the v0.10.x npm dependency.
- **[Risk] The recipe was tested only with Linux's `/bin/sh`.** → It uses only POSIX features. A task tests it on macOS, or records that no Mac was available.
- **[Risk] Contributors used to `just frontend` or the old `just dev` get a surprise.** → `just --list` shows the new names under their stack groups. `server/README.md` and `AGENTS.md` change with the recipes.

## Migration Plan

- After pulling, contributors with Nix get pnpm when direnv reloads the shell. Contributors without Nix run `npm i -g pnpm@12` once.
- Anyone who ran `just frontend` runs `just v0-frontend`. Anyone who wants the full v0.10.x stack runs `just v0-dev`.
- Rollback is a revert of the merge. No data or deployed system changes.
