## Context

See `proposal.md` for the motivation. This section holds only the facts that shape the approach.

- **The image builds from `server/` alone.** `server/Dockerfile` uses `server/` as its build context, so it cannot read `web/`.
- **The server serves no files.** Its router has two routes, `/healthz` and `/readyz`. Every other path returns 404.
- **The foundation already fits a Compose network.** It requires `DATABASE_URL`, runs migrations before it binds HTTP, and accepts `sslmode=disable` for the `db` host.
- **A test fixture already uses Compose.** `server/tests/fixtures/compose.yml` runs PostgreSQL for tests, with fixed test credentials and a loopback port. It is not an installation path.
- **One file pins the PostgreSQL image.** `server/postgres-image.env` holds the exact release and digest for the fixture.
- **Two specs name the image.** `dev-environment` checks `server/Dockerfile` for its Rust base image. `ci` forbids any workflow on `main` from building an image.
- **Nothing reads the storage folder yet.** No server code implements the storage layout.

## Goals / Non-Goals

**Goals:**

- Sam starts a complete instance with `docker compose up` and one required setting.
- The app reuses the foundation's database path with no change to `core`.
- A contributor can verify the whole installation with one local command.
- The verification never touches an instance that Sam already runs on the same machine.

**Non-Goals:**

- Client-side routing. The web shell has one screen, so the server needs no fallback to `index.html`.
- Response caching rules and security headers for the web shell. The authentication work sets the browser security policy.
- A CI job. See decision D9.
- Remapping the app's user ID to match host folders. That belongs with bind mounts in the storage story.

## Decisions

### D1. One root `Dockerfile` with three stages

The `Dockerfile` moves to the repo root, and the build context becomes the repo root.

- **`web` stage**: a `node:26` slim image runs `pnpm install --frozen-lockfile` and `pnpm build` in `web/`.
- **`server` stage**: the existing `rust:1` stage builds `sidereal-server` from `server/`.
- **`runtime` stage**: the existing `debian:bookworm-slim` stage copies the binary and the `web/dist` folder.

The `web` stage installs pnpm 12 with `npm install --global pnpm@12`. pnpm then switches itself to the release that `web/package.json` pins, and checks it against the lockfile.

A root `.dockerignore` starts by excluding everything, then allows `server/` and `web/`. It excludes `server/target`, `web/node_modules`, and `web/dist`. `server/.dockerignore` is removed.

**Why not keep `server/Dockerfile` and add a second image for the web shell?** Two images need a proxy to share one origin. That adds a third service for Sam to run and a second place to set the browser security policy.

**Why not build the web shell on the host and copy it in?** The image would then depend on what the host has installed. A clean clone would not build the same image.

### D2. The server serves files from `WEB_ROOT`

The `server` crate reads the optional `WEB_ROOT` setting. When it is set, the router falls back to a static file service for any path that matches no route.

- The crate uses `tower-http`'s file service. It rejects paths that leave the folder, lists no folders, and answers 405 to methods other than `GET` and `HEAD`.
- Routes win over the fallback, so a file named `healthz` cannot replace the probe.
- The runtime stage sets `WEB_ROOT=/usr/share/sidereal/web`. The folder is owned by root and the app user cannot write to it.
- `just dev` leaves `WEB_ROOT` unset. Vite keeps serving the web shell in development.

**Why not embed the files in the binary?** Embedding makes `cargo build` depend on a finished web build. Every server contributor would need Node, and `just check-server` would need `web/dist`. A folder keeps the two builds separate.

**Why not always serve a fixed folder?** A fixed path would make a development server fail or serve stale files. An optional setting keeps today's behavior when it is absent.

**Where the code lives.** Static serving is HTTP wiring, so it belongs in the `server` crate. `core` gains nothing, and the dependency direction does not change.

### D3. No fallback to `index.html`

An unknown path returns 404. The web shell has one screen and no client-side routes.

**Why not add the fallback now?** A fallback answers every unknown path with 200. Before authentication exists, a 404 for `/api/...` is the clearer signal that no such surface is there. The story that adds the first client-side route adds the fallback with it.

### D4. `WEB_ROOT` is checked before the database

The server validates `WEB_ROOT` first: the folder must exist and hold `index.html`. A broken image then fails before it connects or migrates.

**Why not check after migrations?** A start that cannot serve its UI should not change the database first.

### D5. Compose builds `DATABASE_URL` from one password

`compose.yml` sets fixed names: user `sidereal`, database `sidereal`, host `db`. Only the password comes from Sam.

```
app:  DATABASE_URL=postgres://sidereal:${POSTGRES_PASSWORD}@db:5432/sidereal?sslmode=disable
db:   POSTGRES_PASSWORD=${POSTGRES_PASSWORD:?set POSTGRES_PASSWORD in .env}
```

The `:?` form makes Compose stop when the value is missing or empty. No default appears anywhere.

The guide tells Sam to generate the password with `openssl rand -hex 32`. Hex needs no escaping inside a URL.

**Why not give the server separate host, user, and password settings?** That would be a second connection contract beside the foundation's `DATABASE_URL`. The story forbids it.

**Why not Docker secrets?** The server reads `DATABASE_URL` only. File-based secrets would need new server configuration, for little gain on a single private host.

### D6. Service wiring

- **`db`** runs the pinned PostgreSQL image. Its health check runs `pg_isready` for the `sidereal` database. Its volume mounts at `/var/lib/postgresql`, as the fixture's does.
- **`app`** builds from the root `Dockerfile` and is tagged `sidereal:local`. It waits for `db` with `depends_on: condition: service_healthy`.
- **`app`'s health check** stays on `/healthz`, as the foundation's guidance says. The server binds HTTP only after migrations, so a healthy app has finished them.
- **Both services** use `restart: unless-stopped`.
- **`app`** drops all capabilities and sets `no-new-privileges`. It needs none, because port 5000 is not a privileged port.

### D7. Network surface

`app` publishes `${SIDEREAL_PORT:-5000}:5000` on every host interface, as v0.10.x does. `db` publishes nothing.

The app has no sign-in, so the open port is acceptable only because the surface is tiny: static files and two fixed probes. The spec states that boundary as a requirement, and the guide warns Sam about it.

**Why not bind to loopback by default?** A self-hoster usually opens the app from another device on their network. A loopback default would make the documented first run look broken. The user confirmed this choice during exploration.

### D8. The storage volume is mounted and unused

The runtime stage creates `/var/lib/sidereal/storage`, owned by user ID 10001. A new named volume copies that ownership on first use, so the app user can write to it.

The server sets and reads no storage setting. The storage story adds the setting, bind mounts, and permission checks.

**Why mount it now?** The story's scope includes the volume, and the layout of `compose.yml` then stays stable for Sam when storage arrives.

### D9. Verification is a local recipe

`just instance-verify` runs `scripts/instance-verify.sh`. The script follows the fixture script's pattern for isolation:

- **Project name**: `sidereal-verify-` plus a checksum of the checkout path. Two checkouts never collide, and Sam's default project is untouched.
- **Settings**: a generated `.env` under `.workspace/instance-verify/`, passed with `--env-file`. The root `.env` is never read.
- **Host port**: derived from the same checksum, with `SIDEREAL_VERIFY_PORT` as an override.
- **Cleanup**: an exit trap runs `docker compose down --volumes` for that project only.

The script runs the cheap checks first: the missing-password refusal and the PostgreSQL pin comparison. It builds the image only after they pass.

**Why not add a CI job?** The `ci` spec says no workflow on `main` builds a container image. A CI job would need that requirement changed first. The user chose a local recipe for this story.

**Why not run it from `just check`?** An image build takes minutes and needs Docker. The gate must stay fast and must run for a web-only contributor.

### D10. The PostgreSQL pin is copied, and the recipe compares the copies

`compose.yml` names the image and digest directly. `server/postgres-image.env` stays the place to change it. `just instance-verify` fails when the two differ.

**Why not interpolate `${POSTGRES_IMAGE}`?** Sam would then need two `--env-file` flags on every Compose command. A forgotten flag gives a confusing error on the documented path.

**Why not make `compose.yml` the only source?** The fixture script would need to parse YAML in `sh`.

`CONTRIBUTING.md`'s version table gains two entries: `compose.yml` copies the PostgreSQL pin, and the root `Dockerfile` names the Node major version.

### D11. Design tokens

This change adds no semantic or component token: none.

## Risks / Trade-offs

- **[Nothing in the gate builds the image]** → A change can break the installation and still pass `just check`. Each later story in this epic runs `just instance-verify` in its own verification. A CI job needs a `ci` spec change and is a separate decision.
- **[The PostgreSQL pins can drift]** → Only `just instance-verify` compares them. The version table tells contributors to change both files together.
- **[A changed password locks the app out]** → PostgreSQL reads `POSTGRES_PASSWORD` only when it creates the data folder. The guide states this. Recovery steps belong to the diagnostics story.
- **[A password with URL characters breaks the URL]** → The guide gives a hex command. The foundation then rejects a malformed URL without printing it.
- **[`docker compose config` prints the password]** → This is how Compose works. The guide says so. The app's own logs never print it.
- **[The open port exposes an app with no sign-in]** → The surface is static files and two probes, and a requirement pins it. The guide warns Sam not to publish the port to the internet.
- **[The file service follows symbolic links]** → The image's web folder comes from the build and holds none. The app user cannot write to it.
- **[The first build is slow]** → A release build of the server takes several minutes. Image publishing, which removes this cost for Sam, is out of scope.
- **[`docker build server/` stops working]** → Nothing in the repo calls it. The `dev-environment` spec changes with the file.

## Migration Plan

No released instance uses these files, so no data moves.

1. Add the root `Dockerfile` and `.dockerignore`, then remove `server/Dockerfile` and `server/.dockerignore` in the same commit.
2. Rollback is a revert of the pull request. No database or volume format changes.

## Open Questions

None.
