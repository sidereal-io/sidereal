# compose-instance Specification

## Purpose

Sam can install a private Sidereal instance with Docker Compose and one `.env` file. The app and PostgreSQL start together, and the instance keeps its data when Sam recreates the containers.

## Requirements

### Requirement: One command starts the app and PostgreSQL

The repo root SHALL hold a `compose.yml` that defines two services: `app` and `db`.
`docker compose up -d --build --wait` SHALL build the app image and start both services.
It SHALL exit with status 0 once both are healthy.
The `app` service SHALL start only after the `db` service is healthy.

#### Scenario: A first start reaches readiness

- **GIVEN** a `.env` file that sets `POSTGRES_PASSWORD` to a hex string
- **WHEN** Sam runs `docker compose up -d --build --wait` at the repo root
- **THEN** the command exits with status 0
- **AND** `curl -s http://localhost:5000/readyz` prints `{"status":"ready"}`

#### Scenario: The app starts once on new volumes

- **WHEN** a first start on new volumes has reached readiness
- **AND** a tester reads the `app` container's `RestartCount` with `docker inspect`
- **THEN** the value is `0`

### Requirement: The db service is healthy when PostgreSQL accepts connections

The `db` service SHALL report healthy only when PostgreSQL accepts TCP connections. The app's `/readyz` probe, not this check, reports whether the application database is usable.

#### Scenario: The db service reports healthy

- **GIVEN** a first start that has reached readiness
- **WHEN** a tester reads the `db` container's health status with `docker inspect`
- **THEN** the status is `healthy`
- **AND** `docker compose exec db pg_isready -h 127.0.0.1` exits with status 0

### Requirement: The app connects and migrates through the foundation's startup path

The `app` service SHALL connect to the `db` service with the foundation's `DATABASE_URL` setting and `sslmode=disable`.
The app SHALL apply migrations through the foundation's startup path. The Compose file SHALL run no other migration step.
Storage initialization SHALL run during app startup without a separate command.

#### Scenario: The database holds the initial migration

- **WHEN** a first start has reached readiness
- **AND** a tester counts the rows of `_sqlx_migrations` where `success` is true, through `docker compose exec db psql`
- **THEN** the count is 2

### Requirement: The operator supplies the database password

The bundled entry file `compose.yml` SHALL read the database password from `POSTGRES_PASSWORD`. It SHALL hold no default password.
Bundled mode SHALL refuse to start when `POSTGRES_PASSWORD` is missing or empty.
External mode SHALL NOT require `POSTGRES_PASSWORD`.

#### Scenario: The password is missing

- **GIVEN** a Compose project name that owns no container
- **AND** no `.env` file, and no `POSTGRES_PASSWORD` in the environment
- **WHEN** Sam runs bundled mode with `docker compose up -d` under that project name
- **THEN** the command exits with a non-zero status
- **AND** its output names `POSTGRES_PASSWORD`
- **AND** `docker ps -a -q --filter label=com.docker.compose.project=<name>` exits with status 0 and prints nothing

#### Scenario: The password is empty

- **GIVEN** a Compose project name that owns no container
- **AND** a `.env` file that is an unchanged copy of `.env.example`, and no `POSTGRES_PASSWORD` in the environment
- **WHEN** Sam runs bundled mode with `docker compose up -d` under that project name
- **THEN** the command exits with a non-zero status
- **AND** `docker ps -a -q --filter label=com.docker.compose.project=<name>` exits with status 0 and prints nothing

#### Scenario: The repo holds no password

- **WHEN** a reviewer runs `grep -n '^POSTGRES_PASSWORD=' .env.example`
- **THEN** the output is exactly one line, and nothing follows its `=`
- **AND** `grep -c 'POSTGRES_PASSWORD:-' compose.yml` prints `0`

### Requirement: The example settings file lists every setting

The repo SHALL hold `.env.example`, which lists every supported Compose setting, identifies which mode needs each setting, and leaves `POSTGRES_PASSWORD` empty.
Git SHALL ignore `.env`.

#### Scenario: Git ignores the settings file

- **WHEN** a reviewer runs `git check-ignore .env` at the repo root
- **THEN** the command exits with status 0

### Requirement: The database password uses ASCII letters and digits

The supported password alphabet SHALL be ASCII letters and digits. `.env.example` SHALL state that alphabet beside the setting.
A password outside that alphabet SHALL NOT make the app print the password or the connection URL.

#### Scenario: The password holds URL delimiters

- **GIVEN** new volumes, and `POSTGRES_PASSWORD` set to `Zq7@evil.invalid/x?sslmode=disable#`
- **WHEN** Sam runs bundled mode with `docker compose up -d`, and a tester waits 60 seconds
- **THEN** `docker inspect` reports a health status other than `healthy` for the `app` container
- **AND** the output of `docker compose logs app` contains neither `Zq7@evil` nor `postgres://`

#### Scenario: The example file states the alphabet

- **WHEN** a reviewer reads the comment above `POSTGRES_PASSWORD` in `.env.example`
- **THEN** it says the password must hold only letters and digits

### Requirement: Container logs hold no database password

The logs of both services SHALL NOT contain the database password or the connection URL. This covers a successful start and a failed database login.

#### Scenario: A successful start

- **GIVEN** an instance started with a distinctive 64-character hex password
- **WHEN** a tester searches the output of `docker compose logs` for that password
- **THEN** the search finds no match

#### Scenario: The app has the wrong password

- **GIVEN** an initialized database volume
- **WHEN** Sam starts the instance with a different distinctive `POSTGRES_PASSWORD`
- **THEN** the `app` container exits with a non-zero status
- **AND** the output of `docker compose logs` contains neither password

### Requirement: The server serves the built web shell from a configured folder

The server SHALL read the optional `WEB_ROOT` setting, which names a folder of built web shell files.
With `WEB_ROOT` set, the server SHALL answer `GET` and `HEAD` requests with the matching file under that folder. It SHALL answer `/` with `index.html`.
Without `WEB_ROOT`, the server SHALL serve no files and SHALL answer as it did before this change.

#### Scenario: The home page loads

- **GIVEN** a server running with `WEB_ROOT` set to a built web shell
- **WHEN** a tester runs `curl -si http://localhost:5000/`
- **THEN** the status is 200
- **AND** the `Content-Type` header starts with `text/html`
- **AND** the body equals the `index.html` file in `WEB_ROOT`

#### Scenario: The setting is absent

- **WHEN** Sam starts the server without `WEB_ROOT` and a tester requests `GET /`
- **THEN** the status is 404
- **AND** `GET /healthz` still returns `200 {"status":"ok"}`

### Requirement: The web shell answers 404 for folders and unknown paths

The server SHALL answer 404 for any path other than `/` that names a folder, with or without a trailing slash. It SHALL NOT redirect such a path, and SHALL NOT answer it with that folder's `index.html`.
The server SHALL answer 404 for a path that matches no file. It SHALL NOT answer an unknown path with `index.html`.

#### Scenario: An unknown path is not found

- **GIVEN** a server running with `WEB_ROOT` set to a built web shell
- **WHEN** a tester requests `GET /no-such-page`
- **THEN** the status is 404
- **AND** the body does not equal `index.html`

#### Scenario: A request names a folder

- **GIVEN** a server running with `WEB_ROOT` set to a built web shell that holds an `assets` folder
- **AND** a tester has added the file `assets/index.html` to it
- **WHEN** a tester runs `curl -si http://localhost:5000/assets` and `curl -si http://localhost:5000/assets/`
- **THEN** both statuses are 404
- **AND** neither response has a `Location` header
- **AND** neither body names a file in that folder

### Requirement: An API path is never a web shell path

A path under `/api` is not a web shell path. The API SHALL answer it, as the requirement "The app exposes only static files and health probes" says, and never with a file from `WEB_ROOT`.

#### Scenario: An API path does not fall back to the web shell

- **GIVEN** a server running with `WEB_ROOT` set to a built web shell
- **WHEN** a tester requests `GET /api/assets` with no cookie
- **THEN** the status is 401
- **AND** the body does not equal `index.html`

### Requirement: A request path stays inside WEB_ROOT

The server SHALL NOT let a request path leave `WEB_ROOT`, for example with `..` segments. It SHALL NOT list a folder's contents.

#### Scenario: A path tries to leave the folder

- **GIVEN** a file named `secret.txt` in the parent folder of `WEB_ROOT`
- **WHEN** a tester runs `curl -si --path-as-is http://localhost:5000/../secret.txt` and `curl -si http://localhost:5000/%2e%2e/secret.txt`
- **THEN** neither status is 200
- **AND** neither body contains the contents of `secret.txt`

### Requirement: The web shell folder is trusted and read-only to the app

The server SHALL treat the contents of `WEB_ROOT` as trusted. Whoever can write to that folder decides what the server serves.
The app image SHALL hold the web shell in a folder that the app user cannot write.

#### Scenario: The app user cannot change the web shell

- **GIVEN** a running instance
- **WHEN** a tester runs `docker compose exec app sh -c 'touch "$WEB_ROOT/probe"'`
- **THEN** the command exits with a non-zero status

### Requirement: The health probes win over web shell files

`/healthz` and `/readyz` SHALL keep their documented answers, whatever files `WEB_ROOT` holds.

#### Scenario: A file shadows a health probe

- **GIVEN** `WEB_ROOT` holds files named `healthz` and `readyz`
- **WHEN** a tester requests `GET /healthz` and `GET /readyz` on a ready server
- **THEN** the bodies are `{"status":"ok"}` and `{"status":"ready"}`

### Requirement: The server checks WEB_ROOT before it connects to the database

With `WEB_ROOT` set, the server SHALL check the folder before it connects to the database. It SHALL exit unsuccessfully when the folder is missing or holds no `index.html`.

#### Scenario: The folder is missing

- **GIVEN** an empty dedicated database
- **WHEN** Sam starts the server with `WEB_ROOT` set to a path that does not exist
- **THEN** the process exits with a non-zero status, and its output names `WEB_ROOT`
- **AND** the database still holds no application table

### Requirement: The app image serves the web shell and the health probes on one origin

The app image SHALL hold the server binary, the `siderealctl` binary and the built web shell. It SHALL set `WEB_ROOT` to the web shell's folder.
The browser SHALL reach the home screen and the health probes on the app's own origin. The app SHALL send no `Access-Control-Allow-Origin` header.

#### Scenario: The browser shows the server's health

- **GIVEN** a ready instance
- **WHEN** a user opens `http://localhost:5000/` in a browser
- **THEN** the page shows the text `healthy` within 5 seconds
- **AND** the browser's network log shows no request to any origin other than `http://localhost:5000`

#### Scenario: Another origin asks for the app's health

- **GIVEN** a ready instance
- **WHEN** a tester runs `curl -si -H 'Origin: http://localhost:3000' http://localhost:5000/healthz`
- **THEN** the response headers contain no `Access-Control-Allow-Origin` header

### Requirement: Each Compose project builds its own app image

Compose SHALL name the built image after its Compose project, so two projects on one host never share an image tag.

#### Scenario: Two projects build separate images

- **WHEN** a tester runs `docker compose -p alpha config --images` and `docker compose -p beta config --images`, each with `POSTGRES_PASSWORD` set
- **THEN** the two outputs name different images for the `app` service

### Requirement: The recovery command runs inside the app container

`siderealctl` SHALL be on the `PATH` of the app user, so `docker compose exec app siderealctl reset-admin-password` runs it with the same database settings as the server.

#### Scenario: The recovery command is in the image

- **GIVEN** a ready instance
- **WHEN** a tester runs `docker compose exec -T app siderealctl --help`
- **THEN** the exit status is 0 and the output names `reset-admin-password`

### Requirement: The app exposes only static files and health probes

Without an administrator session, the app SHALL answer only these requests with a status below 400:

- a `GET` or `HEAD` for a web shell file;
- a `GET` or `HEAD` for `/healthz` or `/readyz`;
- the claim and sign-in routes: `GET /api/claim`, `POST /api/claim` while the instance is unclaimed, and `POST /api/session`.

A `POST`, `PUT`, `PATCH` or `DELETE` to a web shell file or a health probe SHALL answer 405.

#### Scenario: A caller sends a mutating request

- **GIVEN** a ready instance
- **WHEN** a caller sends `POST`, `PUT`, `PATCH`, and `DELETE` to `/`, `/healthz`, and `/readyz`
- **THEN** every status is 405
- **AND** the rows of `_sqlx_migrations` and `sidereal_metadata` are unchanged

### Requirement: A caller without a session changes nothing but the claim

No request without a session SHALL change the storage volume, and the only database change open to it is the claim.

#### Scenario: Requests without a session on a claimed instance

- **GIVEN** a ready instance that has been claimed
- **WHEN** a caller with no cookie sends `POST /api/claim`, and `POST`, `PUT`, `PATCH`, and `DELETE` to `/`, `/healthz`, `/readyz`, and `/api/assets`
- **THEN** the rows of `users`, `_sqlx_migrations`, and `sidereal_metadata` are unchanged
- **AND** the storage volume holds the same files as before

### Requirement: Other API routes refuse a caller without a session

Every route under `/api` other than the claim and sign-in routes SHALL answer 401 without a session, or 403 when it is a state-changing request from an origin that is not allowed.

#### Scenario: A caller reaches the API without a session

- **GIVEN** a ready instance
- **WHEN** a caller sends `GET`, `POST`, `PUT`, `PATCH`, and `DELETE` to `/api/assets` with no cookie
- **THEN** every status is 401
- **AND** the rows of `_sqlx_migrations` and `sidereal_metadata` are unchanged

### Requirement: Only the app port is published

The `app` service SHALL publish container port 5000 on host port 5000, on the host address `SIDEREAL_BIND_ADDRESS`, which defaults to `0.0.0.0`, every host interface.
`SIDEREAL_PORT` SHALL change the host port.
`SIDEREAL_BIND_ADDRESS` SHALL change the host address, so that `127.0.0.1` publishes the port to the host alone.
The `db` service SHALL publish no host port. Only the `app` service SHALL reach the database, over the Compose network.

#### Scenario: The database has no host port

- **GIVEN** a running instance
- **WHEN** a tester runs `docker inspect --format '{{json .HostConfig.PortBindings}}'` on the `db` container
- **THEN** the output is `{}`

#### Scenario: The app listens on the default port

- **GIVEN** a running instance with `SIDEREAL_PORT` and `SIDEREAL_BIND_ADDRESS` unset
- **WHEN** a tester runs `docker compose port app 5000`
- **THEN** the output is `0.0.0.0:5000`

#### Scenario: The operator changes the host port

- **GIVEN** a `.env` file that sets `SIDEREAL_PORT=5080`
- **WHEN** Sam starts the instance and runs `curl -s http://localhost:5080/healthz`
- **THEN** the output is `{"status":"ok"}`
- **AND** nothing that the instance started listens on host port 5000

#### Scenario: The operator binds to the host alone

- **GIVEN** a `.env` file that sets `SIDEREAL_BIND_ADDRESS=127.0.0.1`
- **WHEN** a tester runs `docker compose config` and reads the `app` service's `ports`
- **THEN** the one published port has `host_ip` `127.0.0.1` and `published` `5000`

### Requirement: The app runs without root privileges

The `app` container's process SHALL run as user ID 10001. The container SHALL hold no added Linux capabilities and SHALL NOT gain new privileges.

#### Scenario: The app user is not root

- **GIVEN** a running instance
- **WHEN** a tester runs `docker compose exec app id -u`
- **THEN** the output is `10001`

#### Scenario: The container drops its capabilities

- **GIVEN** a running instance
- **WHEN** a tester reads `HostConfig.CapDrop` and `HostConfig.SecurityOpt` with `docker inspect` on the `app` container
- **THEN** `CapDrop` contains `ALL`
- **AND** `SecurityOpt` contains `no-new-privileges:true`

### Requirement: The database keeps its state when containers are recreated

The `db` service SHALL keep its data in a named volume by default.
The database bind override SHALL keep that data at the chosen host directory instead.
`docker compose down` SHALL remove the containers and keep the volume.
After the next start, migration records and the storage pairing SHALL equal their values before removal.
This retention contract SHALL hold for named volumes and supported bind mounts.

#### Scenario: Sam recreates the containers

- **GIVEN** a ready instance, and a saved copy of every `_sqlx_migrations` row's `version`, `checksum`, and `installed_on`
- **WHEN** Sam runs `docker compose down`, then `docker compose up -d --wait`
- **THEN** the same query returns identical rows
- **AND** the storage pairing equals its saved value
- **AND** `curl -s http://localhost:5000/readyz` prints `{"status":"ready"}`

#### Scenario: Removing the containers keeps the volume

- **GIVEN** a ready instance using the default database named volume
- **WHEN** Sam runs `docker compose down`
- **THEN** `docker volume ls` still lists the instance's database volume

### Requirement: The app has a writable storage volume

The `app` service SHALL mount a named volume at `/var/lib/sidereal/storage` by default.
The storage bind override SHALL mount the selected host directory at that same target.
Compose SHALL set `STORAGE_ROOT` to that target.
The app user SHALL be able to create, sync, and delete files there.

#### Scenario: The app user can write to storage

- **GIVEN** a running initialized instance in either supported mount mode
- **WHEN** a tester creates and deletes a probe through `docker compose exec app`
- **THEN** the command exits with status 0

#### Scenario: Storage is a named volume

- **GIVEN** a running initialized instance without a storage bind override
- **WHEN** a tester inspects the app container's mounts
- **THEN** the mount at `/var/lib/sidereal/storage` has type `volume`

### Requirement: Startup checks the storage pairing before serving

The server SHALL validate the marker and database pairing before serving HTTP.
Startup SHALL initialize unpaired storage only under the `managed-storage` rules.
A paired database SHALL NOT initialize an empty substitute store.

#### Scenario: A start leaves storage empty

- **WHEN** a first start on new volumes reaches readiness
- **THEN** storage contains only its identity marker and no image or probe file

### Requirement: The instance uses the pinned PostgreSQL image

The `db` service's image in `compose.yml` SHALL equal the `POSTGRES_IMAGE` value in `server/postgres-image.env`, including its digest.

#### Scenario: The two pins match

- **WHEN** a reviewer compares the `db` service's `image` value in `compose.yml` with the `POSTGRES_IMAGE` value in `server/postgres-image.env`
- **THEN** the two values are the same string

### Requirement: An installation guide tells the operator how to run the instance

The repo SHALL hold the installation guide `docs/install.md`. `README.md` SHALL link to it.
The guide SHALL cover these points:

- the prerequisites: Docker Engine or Docker Desktop, with Compose 2.20 or newer;
- how to create `.env` from `.env.example`, with a command that generates a hex password;
- the one command that builds and starts the instance, including first-start storage initialization;
- the commands that stop and recreate the instance.

#### Scenario: The guide gives a password command

- **WHEN** a reviewer runs `grep -n 'openssl rand -hex 32' docs/install.md`
- **THEN** the search finds at least one match

#### Scenario: The guide names the Compose release

- **WHEN** a reviewer runs `grep -n 'Compose 2.20' docs/install.md`
- **THEN** the search finds at least one match

#### Scenario: The readme links to the guide

- **WHEN** a reviewer runs `grep -n 'docs/install.md' README.md`
- **THEN** the search finds at least one match

### Requirement: The guide tells the operator to claim the instance first

The installation guide `docs/install.md` SHALL cover these points:

- that Sam claims the instance right after the first start, by opening the web page or by running `siderealctl reset-admin-password` first, because the claim is open to the first visitor until then, and how to keep the first start private for the terminal route: set `SIDEREAL_BIND_ADDRESS=127.0.0.1` and keep any reverse-proxy route off until the administrator exists.

#### Scenario: The guide warns about open access

- **WHEN** a reviewer reads `docs/install.md`
- **THEN** it has a section whose heading contains `Claim your instance`, and no section whose heading contains `no sign-in`

### Requirement: The guide shows a reverse proxy with trusted proxies

The installation guide `docs/install.md` SHALL cover these points:

- how to put the instance behind a reverse proxy that terminates TLS, with a Caddy example;
- that the proxy's address, as the app sees it, must be listed in `SIDEREAL_TRUSTED_PROXIES`, and how to find that one address for a proxy on the same machine and for one elsewhere;
- that requests carrying forwarded headers from an unlisted address are refused.

#### Scenario: The guide shows a reverse proxy

- **WHEN** a reviewer reads `docs/install.md`
- **THEN** it has a section whose heading contains `reverse proxy`, which holds a Caddyfile example and names `SIDEREAL_TRUSTED_PROXIES` and `SIDEREAL_BIND_ADDRESS`

### Requirement: The guide explains the bind address and allowed origins behind a proxy

The installation guide `docs/install.md` SHALL cover these points:

- that `SIDEREAL_BIND_ADDRESS=127.0.0.1` keeps the plain-HTTP port off the network when the proxy runs on the same machine;
- that `SIDEREAL_ALLOWED_ORIGINS` is optional and needed only when a proxy rewrites the `Host` header.

#### Scenario: The guide explains when allowed origins are needed

- **WHEN** a reviewer reads the reverse-proxy section of `docs/install.md`
- **THEN** it says that `SIDEREAL_BIND_ADDRESS=127.0.0.1` keeps the plain-HTTP port off the network when the proxy runs on the same machine
- **AND** it names `SIDEREAL_ALLOWED_ORIGINS` as optional, needed only when a proxy rewrites the `Host` header

### Requirement: The guide states the sign-in limits

The installation guide `docs/install.md` SHALL cover these points:

- the fixed sign-in limits: per client address, at most 1 failed sign-in per second, 5 per minute and 20 per hour; that a limited address is answered 429 with `Retry-After`, even for the correct password; that a successful sign-in and a restart clear the count; and that clients behind a proxy that sends no `X-Forwarded-For` share the proxy's address and so one limit.

#### Scenario: The guide states the sign-in limits

- **WHEN** a reviewer reads `docs/install.md`
- **THEN** it has a section whose heading contains `Sign-in limits`, placed after the reverse-proxy section, which names `429`, `Retry-After`, and the limits of 1 per second, 5 per minute and 20 per hour

### Requirement: The guide explains what removing the instance keeps

The installation guide `docs/install.md` SHALL cover these points:

- that `down --volumes` with one bind override deletes only the named-volume data location;
- that retained database pairing with deleted storage refuses startup;
- that an unpaired database adopts retained marker-only storage, but refuses storage containing other files;
- that permanent paired-store loss requires restoring both locations together, or deliberately removing both locations to start fresh.

#### Scenario: The guide distinguishes retained data

- **WHEN** a reviewer reads the installation guide
- **THEN** it names `.env`, the storage marker, the PostgreSQL pairing, and both persistent data locations
- **AND** it states which remain after `docker compose down --volumes`

### Requirement: The guide warns about the web root and a changed password

The installation guide `docs/install.md` SHALL cover these points:

- that a server run outside the image needs a `WEB_ROOT` folder that only trusted users can write;
- that changing `POSTGRES_PASSWORD` after the first start does not change the database's password.

#### Scenario: The guide states the two warnings

- **WHEN** a reviewer reads `docs/install.md`
- **THEN** it says that a server run outside the image needs a `WEB_ROOT` folder that only trusted users can write
- **AND** it says that changing `POSTGRES_PASSWORD` after the first start does not change the database's password

### Requirement: The guide tells the operator how to prepare host directories

The installation guide `docs/install.md` SHALL cover these points:

- how to retain mount selection and host paths in `.env`;
- how to prepare dedicated host directories and grant the app UID/GID `10001:10001` access;
- to select subdirectories below disk mount points and create them only while the intended disk is mounted.

#### Scenario: The guide shows how to prepare a host directory

- **WHEN** a reviewer reads `docs/install.md`
- **THEN** it says how to keep the mount selection and host paths in `.env`
- **AND** it names the UID/GID `10001:10001` and says to choose a subdirectory below the disk's mount point

### Requirement: The guide states the host directory cautions

The installation guide `docs/install.md` SHALL cover these points:

- that an absent subdirectory makes Compose refuse when the disk is unmounted;
- Desktop file sharing, rootless UID mapping, and host ACL or SELinux requirements;
- that PostgreSQL host data needs suitable local database storage;
- that operators must stop containers before remounting.

#### Scenario: The guide warns before remounting

- **WHEN** a reviewer reads `docs/install.md`
- **THEN** it says to stop the containers before remounting a disk
- **AND** it names Desktop file sharing, rootless UID mapping, and host ACL or SELinux requirements

### Requirement: The guide explains what the storage markers prove

The installation guide `docs/install.md` SHALL cover these points:

- that missing markers on paired stores require mount recovery, not reinitialization;
- that startup cannot distinguish simultaneous database and storage loss from first installation;
- that an unpaired database adopts a valid marker-only directory;
- that markers identify stores but do not prove image integrity or detect every live mount change.

#### Scenario: The guide states the limits of the markers

- **WHEN** a reviewer reads `docs/install.md`
- **THEN** it says that a missing marker on a paired store needs mount recovery, not reinitialization
- **AND** it says that markers do not prove image integrity or detect every live mount change

### Requirement: Operators can select persistent host directories independently

The repo SHALL provide `compose.storage-bind.yml` and `compose.database-bind.yml` beside `compose.yml`.
The storage override SHALL require a nonempty `SIDEREAL_STORAGE_PATH`.
The database override SHALL require a nonempty `SIDEREAL_DATABASE_PATH`.
The operator SHALL select overrides through retained `COMPOSE_FILE` configuration or equivalent explicit `-f` arguments.
Both overrides SHALL work independently and together without changing the other service's selected mount.

#### Scenario: Either directory can be chosen independently

- **WHEN** a tester resolves Compose with each override separately and then both together
- **THEN** each selected target has type `bind` and the configured source
- **AND** every unselected target remains a named volume

### Requirement: A bind override changes only where the data lives

Compose SHALL resolve relative host paths against its project directory.
The app SHALL receive only its container-visible storage path, without host-path forwarding.
The app SHALL retain UID/GID `10001:10001` and existing capability restrictions.

#### Scenario: A relative storage path

- **GIVEN** `SIDEREAL_STORAGE_PATH` set to a relative path
- **WHEN** a tester resolves Compose with the storage override
- **THEN** the bind source is that path resolved against the Compose project directory
- **AND** the `app` service's environment names no host path
- **AND** the `app` service keeps user `10001:10001` and its capability restrictions

### Requirement: A selected bind path never falls back to a named volume

The selected bind mounts SHALL use long syntax with `create_host_path: false`.
An invalid selected path SHALL fail rather than select a named volume or create a source directory.

#### Scenario: A selected bind source is absent

- **GIVEN** a selected absolute path that does not exist
- **WHEN** Sam starts the affected service
- **THEN** Compose exits unsuccessfully and the source path still does not exist
- **AND** no fallback volume mounts at that service's selected target

#### Scenario: A selected bind path is empty

- **WHEN** Sam selects a bind override with its path unset or empty
- **THEN** configuration validation or service startup exits unsuccessfully
- **AND** no app HTTP listener starts

### Requirement: Deployment verification demonstrates configuration and storage retention

`mise run instance-verify` SHALL demonstrate retention for default volumes and the combined bind-mount installation.
It SHALL compare database migration records, pairing identity, marker bytes, sentinel bytes, and retained settings across container recreation.
The task SHALL use only its own disposable directories, Compose projects, and credentials.
It SHALL NOT initialize, modify, or delete operator-supplied data paths.

#### Scenario: Recreate both installations

- **WHEN** the verification task runs against default volumes and both bind overrides
- **THEN** it exits successfully only when all saved records and bytes remain identical after recreation
- **AND** the app reaches readiness with the same storage pairing

#### Scenario: The shell holds operator mount settings

- **GIVEN** distinctive operator paths and `COMPOSE_FILE` in the caller's environment
- **WHEN** the verification task runs
- **THEN** those paths remain unchanged and no test container mounts them

### Requirement: Deployment verification demonstrates refusal on empty storage

`mise run instance-verify` SHALL demonstrate refusal when a paired database finds an empty directory at its selected storage target.
The refusal case SHALL confirm that startup failed because the marker is missing.

#### Scenario: A paired database finds empty storage

- **WHEN** the verification task starts a paired database with an empty directory at its selected storage target
- **THEN** the app does not start
- **AND** the task exits successfully only when the startup output says the storage marker is missing

### Requirement: The app's container health follows readiness

The app image's health check SHALL probe `/readyz`, not `/healthz`, every 10 seconds.
Docker SHALL report the `app` container `healthy` after a readiness check passes.
Docker SHALL report it `unhealthy` after three readiness checks in a row fail.
The health status is therefore a sample: Docker can take up to 40 seconds to report a new outage.

#### Scenario: PostgreSQL stops after a ready start

- **GIVEN** a ready instance
- **WHEN** a tester runs `docker compose stop db` and waits 60 seconds
- **THEN** `docker inspect` reports the `app` container's health status as `unhealthy`
- **AND** `curl -s http://localhost:5000/healthz` prints `{"status":"ok"}`
- **AND** `curl -s http://localhost:5000/readyz` prints `{"status":"not_ready"}`

### Requirement: An unhealthy app keeps running and recovers without a restart

An unhealthy app SHALL keep running and keep answering `/healthz`. Docker SHALL NOT restart it because of its health.
The app SHALL report `healthy` again, without a restart, once its database is usable again.
The `db` service's health check SHALL stay unchanged.

#### Scenario: PostgreSQL starts again

- **GIVEN** an instance whose `app` container is `unhealthy` because `db` is stopped
- **WHEN** a tester runs `docker compose start db` and waits 60 seconds
- **THEN** `docker inspect` reports the `app` container's health status as `healthy`
- **AND** the `app` container's `RestartCount` is unchanged
- **AND** the successful `_sqlx_migrations` rows are identical to those saved before the outage

### Requirement: The guide tells the operator how to recover a failed start

`docs/install.md` SHALL have a troubleshooting section.
The section SHALL tell the operator to read `docker compose ps` for health and `docker compose logs app` for the cause.
It SHALL explain that `unhealthy` with a running app means its recent readiness checks failed, and that Docker can take up to 40 seconds to notice an outage.

#### Scenario: The guide has a troubleshooting section

- **WHEN** a reviewer reads `docs/install.md`
- **THEN** it has a section whose heading contains `Troubleshoot`
- **AND** that section names `docker compose logs app` and `docker compose ps`

### Requirement: The troubleshooting section lists each startup message

The troubleshooting section SHALL list each startup and readiness message category with its likely cause and one recovery step.
The list SHALL cover missing or invalid configuration, unreadable certificate files, each connection cause, permission, PostgreSQL version, ownership, migration history, schema, deadlines, migration failure, and storage.

#### Scenario: The guide covers each connection cause

- **WHEN** a reviewer searches the troubleshooting section
- **THEN** it contains `database unreachable`, `database login rejected`, `database TLS verification failed`, and `database does not exist`

### Requirement: The troubleshooting section says when to restart

The troubleshooting section SHALL explain that the app retries startup when Docker restarts it, so a database that starts late needs no action.
It SHALL tell the operator to restart with `docker compose up -d --wait` after fixing `.env`.

#### Scenario: The guide says how to restart

- **WHEN** a reviewer reads the troubleshooting section
- **THEN** it names `docker compose up -d --wait` as the restart after fixing `.env`
- **AND** it says that a database that starts late needs no action

### Requirement: The troubleshooting section rules out destructive recovery

The troubleshooting section SHALL state that deleting volumes or data directories is never a recovery step, and that it destroys the library.
It SHALL state that migration history from a newer release needs that release or a later one, never an older one.
It SHALL state that damaged or unexplained migration history needs investigation with the data kept as it is.
It SHALL state that editing migration records or the schema by hand to force a start is never a recovery step.

#### Scenario: The guide warns against resetting data

- **WHEN** a reviewer reads the troubleshooting section
- **THEN** it states that `docker compose down --volumes` is not a recovery step
- **AND** it states that editing migration records by hand is not a recovery step

### Requirement: Deployment verification demonstrates outage recovery

`mise run instance-verify` SHALL stop the `db` service of a ready default-volume instance.
It SHALL confirm that the `app` container becomes `unhealthy` while `/healthz` still answers `{"status":"ok"}`.
It SHALL start `db` again and confirm that the `app` container becomes `healthy` without a restart.
It SHALL confirm that the app's logs report the outage and the recovery.
It SHALL confirm that the successful migration records and the storage pairing are unchanged after recovery.

#### Scenario: Verify an outage and its recovery

- **WHEN** the verification task runs
- **THEN** it exits successfully only when the app turned `unhealthy`, then `healthy`, with an unchanged `RestartCount`
- **AND** the saved migration records and pairing identity match their values from before the outage

### Requirement: Compose passes the access settings to the app

The `app` service SHALL pass `SIDEREAL_TRUSTED_PROXIES` and `SIDEREAL_ALLOWED_ORIGINS` from the Compose environment to the container, each unset in the container when it is unset in `.env` and the shell. Compose SHALL set no default for either. `.env.example` SHALL show both, commented out, with `SIDEREAL_BIND_ADDRESS`.

#### Scenario: Unset stays unset

- **WHEN** a tester runs `docker compose config` with neither variable set
- **THEN** the `app` service's `environment` holds neither `SIDEREAL_TRUSTED_PROXIES` nor `SIDEREAL_ALLOWED_ORIGINS` with a value

#### Scenario: A value passes through

- **GIVEN** a `.env` file that sets `SIDEREAL_TRUSTED_PROXIES=172.18.0.1` and `SIDEREAL_ALLOWED_ORIGINS=https://photos.example`
- **WHEN** a tester runs `docker compose config`
- **THEN** the `app` service's `environment` holds `SIDEREAL_TRUSTED_PROXIES: 172.18.0.1` and `SIDEREAL_ALLOWED_ORIGINS: https://photos.example`
