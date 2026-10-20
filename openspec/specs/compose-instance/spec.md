# compose-instance Specification

## Purpose

Sam can install a private Sidereal instance with Docker Compose and one `.env` file. The app and PostgreSQL start together, and the instance keeps its data when Sam recreates the containers.

## Requirements

### Requirement: One command starts the app and PostgreSQL

The repo root SHALL hold a `compose.yml` that defines two services: `app` and `db`.
`docker compose up -d --build --wait` SHALL build the app image and start both services.
It SHALL exit with status 0 once both are healthy.
The `app` service SHALL start only after the `db` service is healthy.
The `db` service SHALL report healthy only when PostgreSQL accepts TCP connections. The app's `/readyz` probe, not this check, reports whether the application database is usable.
The `app` service SHALL connect to the `db` service with the foundation's `DATABASE_URL` setting and `sslmode=disable`.
The app SHALL apply migrations through the foundation's startup path. The Compose file SHALL run no other migration step.
Storage initialization SHALL run during app startup without a separate command.

#### Scenario: A first start reaches readiness

- **GIVEN** a `.env` file that sets `POSTGRES_PASSWORD` to a hex string
- **WHEN** Sam runs `docker compose up -d --build --wait` at the repo root
- **THEN** the command exits with status 0
- **AND** `curl -s http://localhost:5000/readyz` prints `{"status":"ready"}`

#### Scenario: The database holds the initial migration

- **WHEN** a first start has reached readiness
- **AND** a tester counts the rows of `_sqlx_migrations` where `success` is true, through `docker compose exec db psql`
- **THEN** the count is 2

#### Scenario: The app starts once on new volumes

- **WHEN** a first start on new volumes has reached readiness
- **AND** a tester reads the `app` container's `RestartCount` with `docker inspect`
- **THEN** the value is `0`

### Requirement: The operator supplies the database password

The Compose file SHALL read the database password from `POSTGRES_PASSWORD`. It SHALL hold no default password.
Compose SHALL refuse to start when `POSTGRES_PASSWORD` is missing or empty.
The repo SHALL hold `.env.example`, which lists every Compose setting and leaves `POSTGRES_PASSWORD` empty.
Git SHALL ignore `.env`.
The supported password alphabet SHALL be ASCII letters and digits. `.env.example` SHALL state that alphabet beside the setting.
A password outside that alphabet SHALL NOT make the app print the password or the connection URL.

#### Scenario: The password is missing

- **GIVEN** a Compose project name that owns no container
- **AND** no `.env` file, and no `POSTGRES_PASSWORD` in the environment
- **WHEN** Sam runs `docker compose up -d` under that project name
- **THEN** the command exits with a non-zero status
- **AND** its output names `POSTGRES_PASSWORD`
- **AND** `docker ps -a -q --filter label=com.docker.compose.project=<name>` exits with status 0 and prints nothing

#### Scenario: The password is empty

- **GIVEN** a Compose project name that owns no container
- **AND** a `.env` file that is an unchanged copy of `.env.example`, and no `POSTGRES_PASSWORD` in the environment
- **WHEN** Sam runs `docker compose up -d` under that project name
- **THEN** the command exits with a non-zero status
- **AND** `docker ps -a -q --filter label=com.docker.compose.project=<name>` exits with status 0 and prints nothing

#### Scenario: The repo holds no password

- **WHEN** a reviewer runs `grep -n '^POSTGRES_PASSWORD=' .env.example`
- **THEN** the output is exactly one line, and nothing follows its `=`
- **AND** `grep -c 'POSTGRES_PASSWORD:-' compose.yml` prints `0`

#### Scenario: The password holds URL delimiters

- **GIVEN** new volumes, and `POSTGRES_PASSWORD` set to `Zq7@evil.invalid/x?sslmode=disable#`
- **WHEN** Sam runs `docker compose up -d`, and a tester waits 60 seconds
- **THEN** `docker inspect` reports a health status other than `healthy` for the `app` container
- **AND** the output of `docker compose logs app` contains neither `Zq7@evil` nor `postgres://`

#### Scenario: The example file states the alphabet

- **WHEN** a reviewer reads the comment above `POSTGRES_PASSWORD` in `.env.example`
- **THEN** it says the password must hold only letters and digits

#### Scenario: Git ignores the settings file

- **WHEN** a reviewer runs `git check-ignore .env` at the repo root
- **THEN** the command exits with status 0

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
The server SHALL answer 404 for any other path that names a folder, with or without a trailing slash. It SHALL NOT redirect such a path, and SHALL NOT answer it with that folder's `index.html`.
The server SHALL answer 404 for a path that matches no file. It SHALL NOT answer an unknown path with `index.html`.
The server SHALL NOT let a request path leave `WEB_ROOT`, for example with `..` segments. It SHALL NOT list a folder's contents.
The server SHALL treat the contents of `WEB_ROOT` as trusted. Whoever can write to that folder decides what the server serves.
`/healthz` and `/readyz` SHALL keep their documented answers, whatever files `WEB_ROOT` holds.
With `WEB_ROOT` set, the server SHALL check the folder before it connects to the database. It SHALL exit unsuccessfully when the folder is missing or holds no `index.html`.
The app image SHALL hold the web shell in a folder that the app user cannot write.
Without `WEB_ROOT`, the server SHALL serve no files and SHALL answer as it did before this change.

#### Scenario: The home page loads

- **GIVEN** a server running with `WEB_ROOT` set to a built web shell
- **WHEN** a tester runs `curl -si http://localhost:5000/`
- **THEN** the status is 200
- **AND** the `Content-Type` header starts with `text/html`
- **AND** the body equals the `index.html` file in `WEB_ROOT`

#### Scenario: An unknown path is not found

- **GIVEN** a server running with `WEB_ROOT` set to a built web shell
- **WHEN** a tester requests `GET /no-such-page` and `GET /api/assets`
- **THEN** both statuses are 404
- **AND** neither body equals `index.html`

#### Scenario: A path tries to leave the folder

- **GIVEN** a file named `secret.txt` in the parent folder of `WEB_ROOT`
- **WHEN** a tester runs `curl -si --path-as-is http://localhost:5000/../secret.txt` and `curl -si http://localhost:5000/%2e%2e/secret.txt`
- **THEN** neither status is 200
- **AND** neither body contains the contents of `secret.txt`

#### Scenario: A request names a folder

- **GIVEN** a server running with `WEB_ROOT` set to a built web shell that holds an `assets` folder
- **AND** a tester has added the file `assets/index.html` to it
- **WHEN** a tester runs `curl -si http://localhost:5000/assets` and `curl -si http://localhost:5000/assets/`
- **THEN** both statuses are 404
- **AND** neither response has a `Location` header
- **AND** neither body names a file in that folder

#### Scenario: The app user cannot change the web shell

- **GIVEN** a running instance
- **WHEN** a tester runs `docker compose exec app sh -c 'touch "$WEB_ROOT/probe"'`
- **THEN** the command exits with a non-zero status

#### Scenario: A file shadows a health probe

- **GIVEN** `WEB_ROOT` holds files named `healthz` and `readyz`
- **WHEN** a tester requests `GET /healthz` and `GET /readyz` on a ready server
- **THEN** the bodies are `{"status":"ok"}` and `{"status":"ready"}`

#### Scenario: The folder is missing

- **GIVEN** an empty dedicated database
- **WHEN** Sam starts the server with `WEB_ROOT` set to a path that does not exist
- **THEN** the process exits with a non-zero status, and its output names `WEB_ROOT`
- **AND** the database still holds no application table

#### Scenario: The setting is absent

- **WHEN** Sam starts the server without `WEB_ROOT` and a tester requests `GET /`
- **THEN** the status is 404
- **AND** `GET /healthz` still returns `200 {"status":"ok"}`

### Requirement: The app image serves the web shell and the health probes on one origin

The app image SHALL hold the server binary and the built web shell. It SHALL set `WEB_ROOT` to the web shell's folder.
Compose SHALL name the built image after its Compose project, so two projects on one host never share an image tag.
The browser SHALL reach the home screen and the health probes on the app's own origin. The app SHALL send no `Access-Control-Allow-Origin` header.

#### Scenario: The browser shows the server's health

- **GIVEN** a ready instance
- **WHEN** a user opens `http://localhost:5000/` in a browser
- **THEN** the page shows the text `healthy` within 5 seconds
- **AND** the browser's network log shows no request to any origin other than `http://localhost:5000`

#### Scenario: Two projects build separate images

- **WHEN** a tester runs `docker compose -p alpha config --images` and `docker compose -p beta config --images`, each with `POSTGRES_PASSWORD` set
- **THEN** the two outputs name different images for the `app` service

#### Scenario: Another origin asks for the app's health

- **GIVEN** a ready instance
- **WHEN** a tester runs `curl -si -H 'Origin: http://localhost:3000' http://localhost:5000/healthz`
- **THEN** the response headers contain no `Access-Control-Allow-Origin` header

### Requirement: The app exposes only static files and health probes

Until authentication exists, the app SHALL answer only two kinds of request with a status below 400:

- a `GET` or `HEAD` for a web shell file;
- a `GET` or `HEAD` for `/healthz` or `/readyz`.

The app SHALL answer every `POST`, `PUT`, `PATCH`, and `DELETE` request with 405.
No request SHALL change the database or the storage volume.

#### Scenario: A caller sends a mutating request

- **GIVEN** a ready instance
- **WHEN** a caller sends `POST`, `PUT`, `PATCH`, and `DELETE` to `/`, `/healthz`, `/readyz`, and `/api/assets`
- **THEN** every status is 405
- **AND** the rows of `_sqlx_migrations` and `sidereal_metadata` are unchanged

### Requirement: Only the app port is published

The `app` service SHALL publish container port 5000 on host port 5000, on every host interface.
`SIDEREAL_PORT` SHALL change the host port.
The `db` service SHALL publish no host port. Only the `app` service SHALL reach the database, over the Compose network.

#### Scenario: The database has no host port

- **GIVEN** a running instance
- **WHEN** a tester runs `docker inspect --format '{{json .HostConfig.PortBindings}}'` on the `db` container
- **THEN** the output is `{}`

#### Scenario: The app listens on the default port

- **GIVEN** a running instance with `SIDEREAL_PORT` unset
- **WHEN** a tester runs `docker compose port app 5000`
- **THEN** the output is `0.0.0.0:5000`

#### Scenario: The operator changes the host port

- **GIVEN** a `.env` file that sets `SIDEREAL_PORT=5080`
- **WHEN** Sam starts the instance and runs `curl -s http://localhost:5080/healthz`
- **THEN** the output is `{"status":"ok"}`
- **AND** nothing that the instance started listens on host port 5000

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
The server SHALL validate the marker and database pairing before serving HTTP.
Startup SHALL initialize unpaired storage only under the `managed-storage` rules.
A paired database SHALL NOT initialize an empty substitute store.

#### Scenario: The app user can write to storage

- **GIVEN** a running initialized instance in either supported mount mode
- **WHEN** a tester creates and deletes a probe through `docker compose exec app`
- **THEN** the command exits with status 0

#### Scenario: Storage is a named volume

- **GIVEN** a running initialized instance without a storage bind override
- **WHEN** a tester inspects the app container's mounts
- **THEN** the mount at `/var/lib/sidereal/storage` has type `volume`

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
- the commands that stop and recreate the instance;
- that `down --volumes` with one bind override deletes only the named-volume data location;
- that retained database pairing with deleted storage refuses startup;
- that an unpaired database adopts retained marker-only storage, but refuses storage containing other files;
- that permanent paired-store loss requires restoring both locations together, or deliberately removing both locations to start fresh;
- that the app has no sign-in yet, so anyone who can reach its port can open it;
- that a server run outside the image needs a `WEB_ROOT` folder that only trusted users can write;
- that changing `POSTGRES_PASSWORD` after the first start does not change the database's password;
- how to retain mount selection and host paths in `.env`;
- how to prepare dedicated host directories and grant the app UID/GID `10001:10001` access;
- to select subdirectories below disk mount points and create them only while the intended disk is mounted;
- that an absent subdirectory makes Compose refuse when the disk is unmounted;
- Desktop file sharing, rootless UID mapping, and host ACL or SELinux requirements;
- that PostgreSQL host data needs suitable local database storage;
- that operators must stop containers before remounting;
- that missing markers on paired stores require mount recovery, not reinitialization;
- that startup cannot distinguish simultaneous database and storage loss from first installation;
- that an unpaired database adopts a valid marker-only directory;
- that markers identify stores but do not prove image integrity or detect every live mount change.

#### Scenario: The guide gives a password command

- **WHEN** a reviewer runs `grep -n 'openssl rand -hex 32' docs/install.md`
- **THEN** the search finds at least one match

#### Scenario: The guide warns about open access

- **WHEN** a reviewer reads `docs/install.md`
- **THEN** it has a section whose heading contains `no sign-in`

#### Scenario: The guide names the Compose release

- **WHEN** a reviewer runs `grep -n 'Compose 2.20' docs/install.md`
- **THEN** the search finds at least one match

#### Scenario: The readme links to the guide

- **WHEN** a reviewer runs `grep -n 'docs/install.md' README.md`
- **THEN** the search finds at least one match

#### Scenario: The guide distinguishes retained data

- **WHEN** a reviewer reads the installation guide
- **THEN** it names `.env`, the storage marker, the PostgreSQL pairing, and both persistent data locations
- **AND** it states which remain after `docker compose down --volumes`

### Requirement: Operators can select persistent host directories independently

The repo SHALL provide `compose.storage-bind.yml` and `compose.database-bind.yml` beside `compose.yml`.
The storage override SHALL require a nonempty `SIDEREAL_STORAGE_PATH`.
The database override SHALL require a nonempty `SIDEREAL_DATABASE_PATH`.
Compose SHALL resolve relative host paths against its project directory.
The app SHALL receive only its container-visible storage path, without host-path forwarding.
The selected bind mounts SHALL use long syntax with `create_host_path: false`.
The operator SHALL select overrides through retained `COMPOSE_FILE` configuration or equivalent explicit `-f` arguments.
An invalid selected path SHALL fail rather than select a named volume or create a source directory.
Both overrides SHALL work independently and together without changing the other service's selected mount.
The app SHALL retain UID/GID `10001:10001` and existing capability restrictions.

#### Scenario: Either directory can be chosen independently

- **WHEN** a tester resolves Compose with each override separately and then both together
- **THEN** each selected target has type `bind` and the configured source
- **AND** every unselected target remains a named volume

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

`just instance-verify` SHALL demonstrate retention for default volumes and the combined bind-mount installation.
It SHALL compare database migration records, pairing identity, marker bytes, sentinel bytes, and retained settings across container recreation.
It SHALL demonstrate refusal when a paired database finds an empty directory at its selected storage target.
The refusal case SHALL confirm that startup failed because the marker is missing.
The recipe SHALL use only its own disposable directories, Compose projects, and credentials.
It SHALL NOT initialize, modify, or delete operator-supplied data paths.

#### Scenario: Recreate both installations

- **WHEN** the verification recipe runs against default volumes and both bind overrides
- **THEN** it exits successfully only when all saved records and bytes remain identical after recreation
- **AND** the app reaches readiness with the same storage pairing

#### Scenario: The shell holds operator mount settings

- **GIVEN** distinctive operator paths and `COMPOSE_FILE` in the caller's environment
- **WHEN** the verification recipe runs
- **THEN** those paths remain unchanged and no test container mounts them
