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
- The crate turns off the file service's folder handling, which would redirect `/assets` and serve a folder's `index.html`. An explicit route serves `/` from `index.html`. Every other folder path returns 404.
- The file service follows symbolic links, as most web servers do. The server trusts what the folder holds. Whoever can write to it already controls the page the browser loads.
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

The server validates `WEB_ROOT` first. A broken image then fails before it connects or migrates. The check has two rules: the folder exists, and it holds a file named `index.html`.

**Why not check after migrations?** A start that cannot serve its UI should not change the database first.

**Why not refuse symbolic links?** A check at startup cannot cover a link that someone adds later. Guarding every file open adds code for a case the operator already controls. In the image, root owns the web folder and the app user cannot write to it. For a server run by hand, the guide says who may write to the folder.

### D5. Compose builds `DATABASE_URL` from one password

`compose.yml` sets fixed names: user `sidereal`, database `sidereal`, host `db`. Only the password comes from Sam.

```
app:  DATABASE_URL=postgres://sidereal:${POSTGRES_PASSWORD}@db:5432/sidereal?sslmode=disable
db:   POSTGRES_PASSWORD=${POSTGRES_PASSWORD:?set POSTGRES_PASSWORD in .env}
```

The `:?` form makes Compose stop when the value is missing or empty. No default appears anywhere.

The guide tells Sam to generate the password with `openssl rand -hex 32`. Hex needs no escaping inside a URL.

Compose cannot check a value's characters, so the plan does not enforce the alphabet. The guide and `.env.example` state it: letters and digits only.
A password with URL delimiters, such as `@` or `/`, changes what the URL means. The app then fails to start, and the foundation's redaction keeps the value out of the logs.
This is not a trust boundary. The person who sets the password already controls `compose.yml` and the host.

**Why not give the server separate host, user, and password settings?** That would be a second connection contract beside the foundation's `DATABASE_URL`. The story forbids it.

**Why not Docker secrets?** The server reads `DATABASE_URL` only. File-based secrets would need new server configuration, for little gain on a single private host.

### D6. Service wiring

- **`db`** runs the pinned PostgreSQL image. Its named volume `database` mounts at `/var/lib/postgresql`, as the fixture's does.
- **`db`'s health check** runs `pg_isready` against `127.0.0.1`. It reports that the server accepts TCP connections, and nothing about the `sidereal` database. The app's `/readyz` covers that. During first setup, the image runs a temporary server on a socket only. A TCP check stays unhealthy until the real server listens.
- **`app`** builds from the root `Dockerfile`. `compose.yml` sets no `image` name, so Compose tags the image `<project>-app`. A second project on the same host then builds its own image.
- **`app`** waits for `db` with `depends_on: condition: service_healthy`.
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

- **Project name**: `sidereal-verify-` plus a 16-character SHA-256 prefix of the checkout path. A SHA-256 prefix makes a shared name impractical. Sam's default project is untouched.
- **Ownership**: the name alone proves nothing: another checkout could still hold it. The script records the project name in its lock folder when it starts an instance. It then stops, and removes nothing, when Docker lists a container or volume under that name and its own lock folder holds no record of creating it.
- **Image**: Compose tags it after the project, so the script never rebuilds the image that Sam's instance uses.
- **Settings**: the script exports its own `POSTGRES_PASSWORD` and `SIDEREAL_PORT` for every Compose call. A shell value outranks an env file, so exported values also outrank whatever the caller's shell held. It passes an empty env file with `--env-file`, so Compose never reads the root `.env`.
- **Host port**: derived from a 32-bit checksum of the checkout path, with `SIDEREAL_VERIFY_PORT` as an override.
- **Deadlines**: the script runs each `docker compose up --wait` in the background and measures 180 seconds itself. Compose replaces the background subshell (`run=exec`), so a kill reaches Compose directly. It stops an overdue call, then fails. Compose's own `--wait-timeout` does not cover the wait for a dependency on every release, so the script does not rely on it. Every probe loop has a fixed number of tries.
- **One run per checkout**: the script creates a lock folder under `.workspace/instance-verify/` before it touches Docker. Creating a folder is atomic. The lock holds the run's process ID. A second folder guards replacement so only one run at a time can replace the lock of a run that died, and the winner verifies the process ID again before replacing the lock.
- **Compose release**: the script reads `docker compose version --short` and stops when the release is older than 2.20. That release is the stated floor. The plan tests only the release the contributor has installed.
- **Cleanup**: an exit trap kills and waits for any running Compose command, and runs `docker compose down --volumes --rmi local` for that project only.

The script runs the cheap checks first: the missing-password refusal and the PostgreSQL pin comparison. It runs the refusal check with `POSTGRES_PASSWORD` unset. It builds the image only after both pass.

**Order of the volume checks.** The script inspects the `db` container's mounts before the first `docker compose down`. The data folder must sit on the project's named `database` volume.
The PostgreSQL image declares its data folder as a volume. Without a named mount, Docker creates an unnamed volume, and a plain `down` would orphan it. When the mount check fails, the script stops while the container still exists. The trap's `down --volumes` then removes the unnamed volume with it.

**Why not add a CI job?** The `ci` spec says no workflow on `main` builds a container image. A CI job would need that requirement changed first. The user chose a local recipe for this story.

**Why not run it from `just check`?** An image build takes minutes and needs Docker. The gate must stay fast and must run for a web-only contributor.

### D10. The PostgreSQL pin is copied, and the recipe compares the copies

`compose.yml` names the image and digest directly. `server/postgres-image.env` stays the place to change it. `just instance-verify` fails when the two differ.

**Why not interpolate `${POSTGRES_IMAGE}`?** Sam would then need two `--env-file` flags on every Compose command. A forgotten flag gives a confusing error on the documented path.

**Why not make `compose.yml` the only source?** The fixture script would need to parse YAML in `sh`.

`CONTRIBUTING.md`'s version table gains two entries: `compose.yml` copies the PostgreSQL pin, and the root `Dockerfile` names the Node major version.

### D11. Design tokens

None. This change adds no token.

## Risks / Trade-offs

- **[Nothing in the gate builds the image]** → A change can break the installation and still pass `just check`. Each later story in this epic runs `just instance-verify` in its own verification. A CI job needs a `ci` spec change and is a separate decision.
- **[The PostgreSQL pins can drift]** → Only `just instance-verify` compares them. The version table tells contributors to change both files together.
- **[A changed password locks the app out]** → PostgreSQL reads `POSTGRES_PASSWORD` only when it creates the data folder. The guide states this. Recovery steps belong to the diagnostics story.
- **[A password with URL characters breaks the URL]** → The guide gives a hex command. The foundation then rejects a malformed URL without printing it.
- **[`docker compose config` prints the password]** → This is how Compose works. The guide says so. The app's own logs never print it.
- **[The open port exposes an app with no sign-in]** → The surface is static files and two probes, and a requirement pins it. The guide warns Sam not to publish the port to the internet.
- **[The file service follows symbolic links]** → A link in the web folder can expose a file outside it. The app user cannot write to the image's web folder. The guide tells an operator who runs the server by hand to keep the folder writable by trusted users only.
- **[The Compose floor is stated, not proven]** → Nobody has run the plan on Compose 2.20. If an older or newer release breaks a command, the fix is to raise the floor or change that command.
- **[Two checkouts can share one project name]** → Compose names a project after its folder. Two checkouts in folders with the same name share containers, volumes, and the image. The guide tells Sam to run one instance per host, or to set `COMPOSE_PROJECT_NAME`.
- **[The first build is slow]** → A release build of the server takes several minutes. Image publishing, which removes this cost for Sam, is out of scope.
- **[`docker build server/` stops working]** → Nothing in the repo calls it. The `dev-environment` spec changes with the file.

## Migration Plan

No released instance uses these files, so no data moves.

1. Add the root `Dockerfile` and `.dockerignore`, then remove `server/Dockerfile` and `server/.dockerignore` in the same commit.
2. Rollback is a revert of the pull request. No database or volume format changes.

## Open Questions

None.
