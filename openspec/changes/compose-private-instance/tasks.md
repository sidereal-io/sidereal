## 1. Serve the web shell from the server

- [x] 1.1 Add `tower-http` with its file-serving feature to `server/Cargo.toml` and the `server` crate. Verify `cargo build -p sidereal-server` succeeds and `server/scripts/check-arch.sh` passes.
- [x] 1.2 Read the optional `WEB_ROOT` setting in `server/crates/server/src/config.rs`. Reject a missing folder and a folder with no `index.html`, with an error that names `WEB_ROOT`. Verify with unit tests for the unset, valid, missing-folder, and missing-index cases.
- [x] 1.3 Check `WEB_ROOT` in `main.rs` before the database initializes. Verify with a test in `server/crates/server/tests/` that a bad `WEB_ROOT` exits non-zero and leaves an empty database with no application table.
- [x] 1.4 Build the router with an optional web root in `lib.rs`: an explicit route for `/`, and a file-service fallback with folder redirects and folder index files turned off. Verify with tests that `GET /` returns `index.html` as `text/html`, and that a file under `assets/` returns its bytes.
- [x] 1.5 Add router tests for the limits of the surface. Verify these cases pass: an unknown path and `/api/assets` return 404; `/assets` and `/assets/` return 404 with no `Location` header when `assets/index.html` exists; `..` and `%2e%2e` paths never return a file outside the folder; `POST`, `PUT`, `PATCH`, and `DELETE` return 405 on `/`, `/healthz`, `/readyz`, and `/api/assets`.
- [x] 1.6 Add router tests that files named `healthz` and `readyz` in the web root do not replace the probes, and that no response carries `Access-Control-Allow-Origin`. Verify the tests pass.
- [x] 1.7 Confirm the server is unchanged without `WEB_ROOT`. Verify the existing `healthz.rs` and `database.rs` tests pass, and a new test shows `GET /` returns 404.
- [x] 1.8 Run `just check-server`. Verify it exits with status 0.

## 2. Build one app image

- [x] 2.1 Add a root `.dockerignore` that excludes everything, then allows `server/` and `web/` without `server/target`, `web/node_modules`, and `web/dist`. Verify `docker build` sends a context of a few megabytes, not the whole checkout.
- [x] 2.2 Add the root `Dockerfile` with the `web`, `server`, and `runtime` stages from design D1, and remove `server/Dockerfile` and `server/.dockerignore` in the same commit. Verify `docker build .` exits with status 0.
- [x] 2.3 In the `web` stage, install pnpm 12 and run `pnpm install --frozen-lockfile` and `pnpm build`. Verify the build log shows the pnpm release that `web/package.json` pins, and the stage holds `web/dist/index.html`.
- [x] 2.4 In the `runtime` stage, copy the web shell to `/usr/share/sidereal/web` owned by root, set `WEB_ROOT`, and create `/var/lib/sidereal/storage` owned by user ID 10001. Verify with `docker run` that `id -u` prints `10001`, that `touch "$WEB_ROOT/probe"` fails, and that `touch /var/lib/sidereal/storage/probe` succeeds.
- [x] 2.5 Check the base image rules. Verify every `FROM rust:` line starts with `rust:1-` and every `FROM node:` line starts with `node:26-`.

## 3. Add the Compose installation

- [x] 3.1 Add the root `compose.yml` with the `db` service: the pinned image copied from `server/postgres-image.env`, the named volume `database` at `/var/lib/postgresql`, a `pg_isready` health check against `127.0.0.1`, `restart: unless-stopped`, and no published port. Verify `docker compose config` succeeds with `POSTGRES_PASSWORD` set.
- [x] 3.2 Add the `app` service: `build: .` with no `image` name, `depends_on` with `service_healthy`, the composed `DATABASE_URL`, the port `${SIDEREAL_PORT:-5000}:5000`, the named volume `storage`, dropped capabilities, `no-new-privileges`, and `restart: unless-stopped`. Verify `docker compose -p alpha config --images` and `-p beta config --images` name different app images.
- [x] 3.3 Require the password with the `:?` form in both services. Verify `docker compose up -d` exits non-zero and names `POSTGRES_PASSWORD` when it is unset and when it is empty, and that `docker ps -a -q --filter label=com.docker.compose.project=<name>` prints nothing.
- [x] 3.4 Add `.env.example` with every Compose setting, an empty `POSTGRES_PASSWORD`, and a comment that states the letters-and-digits alphabet. Verify `grep -n '^POSTGRES_PASSWORD=' .env.example` prints one line with nothing after `=`, and `git check-ignore .env` exits with status 0.
- [x] 3.5 Start the instance by hand with a generated password and `docker compose up -d --build --wait`. Verify `/readyz` prints `{"status":"ready"}`, the home page shows `healthy` in a browser, and the `app` container's `RestartCount` is `0`.

## 4. Add the verification recipe

- [x] 4.1 Add `scripts/instance-verify.sh` with its setup: the Docker and Compose 2.20 checks, the lock folder under `.workspace/instance-verify/`, the project name, the ownership check, the exported password and port, the empty env file, and the exit trap that runs `down --volumes --rmi local`. Verify the script exits non-zero and names Docker when the daemon is unreachable.
- [x] 4.2 Add the checks that need no build: Compose refuses a missing password, and the `db` image equals the pin in `server/postgres-image.env`. Verify a changed image tag in `compose.yml` fails the script before any build starts.
- [x] 4.3 Add the build and the first start, with the script's own 180-second limit around `docker compose up --wait`. Verify a `db` health check changed to `false` fails the script within 240 seconds and leaves no container or volume.
- [x] 4.4 Add the instance checks: the home page, `/healthz`, `/readyz`, user ID 10001, no `db` host port, the writable storage volume, and the named `database` mount. Run the mount check before any `docker compose down`. Verify that removing the `db` volume mount fails the script and leaves no new volume in `docker volume ls -q`.
- [x] 4.5 Add the retention check: save the migration rows, run `docker compose down`, start again, and compare the rows. Verify the script passes on an unchanged checkout.
- [x] 4.6 Add the secret checks: search the logs for the generated password after the first start, then start the app with a different password and search for both. Verify the script fails when a test line echoes the password into a container's log.
- [x] 4.7 Add the `instance-verify` recipe to the root `justfile`, outside `check`. Verify `just --show check` still prints `check: check-server check-web`, and `just --list` describes the new recipe.
- [x] 4.8 Test the isolation rules. Verify a second run in the same checkout exits non-zero within 10 seconds while the first run passes; a run with `POSTGRES_PASSWORD=shellsecret` and `SIDEREAL_PORT=5000` exported keeps both out of the disposable instance; and a container with the project label made outside the checkout stops the script without being removed.
- [x] 4.9 Run `just instance-verify` beside an instance started from the repo root. Verify the instance keeps its container IDs, volumes, and app image ID, and `.env` and `git status --porcelain` are unchanged.

## 5. Document the installation

- [x] 5.1 Write `docs/install.md` in plain language with every point the `compose-instance` spec lists, including a section whose heading contains `no sign-in`. Verify `grep -n 'openssl rand -hex 32' docs/install.md` and `grep -n 'Compose 2.20' docs/install.md` each find a match.
- [x] 5.2 Link the guide from `README.md`. Verify `grep -n 'docs/install.md' README.md` finds a match.
- [x] 5.3 Update `server/README.md`: add `WEB_ROOT` to the settings table, and replace any mention of `server/Dockerfile`. Verify the table lists `WEB_ROOT` with its default and purpose.
- [x] 5.4 Update the version table in `CONTRIBUTING.md`: `compose.yml` copies the PostgreSQL pin, and the root `Dockerfile` names the Node major version. Add `just instance-verify` to the command list. Verify both table rows name the new files.
- [x] 5.5 Post the pre-authentication scope note on issue #389, after the owner approves its text. It states that the instance serves only static files and two health probes until authentication lands. Verify the comment appears on the issue.

## 6. Verify the whole change

- [x] 6.1 Run `just check`. Verify it exits with status 0 and `git status --porcelain` prints nothing afterwards.
- [x] 6.2 Run `just instance-verify` on the finished branch. Verify it exits with status 0 and leaves no container, volume, or image under its project name.
- [x] 6.3 Run the browser demonstration from the story: open `http://localhost:5000/`, confirm `healthy`, recreate the containers with `docker compose down` and `docker compose up -d --wait`, and confirm the migration rows are identical. Record the commands and their output in the pull request description.
- [x] 6.4 Run `openspec validate compose-private-instance --strict`. Verify the change is valid.
