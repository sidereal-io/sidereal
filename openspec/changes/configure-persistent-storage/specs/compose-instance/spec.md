## MODIFIED Requirements

### Requirement: One command starts the app and PostgreSQL

The repo root SHALL hold a `compose.yml` that defines two services: `app` and `db`.
After explicit storage initialization, `docker compose up -d --build --wait` SHALL build the app image, start both services, and exit with status 0 once both are healthy.
The `app` service SHALL start only after the `db` service is healthy.
The `db` service SHALL report healthy only when PostgreSQL accepts TCP connections. The app's `/readyz` probe, not this check, reports whether the application database is usable.
The `app` service SHALL connect to the `db` service with the foundation's `DATABASE_URL` setting and `sslmode=disable`.
The app SHALL apply migrations through the foundation's startup path. The Compose file SHALL run no other migration step.
The explicit storage initialization command SHALL reuse that same migration path.

#### Scenario: A first start reaches readiness

- **GIVEN** a `.env` file that sets `POSTGRES_PASSWORD` to a hex string
- **AND** Sam has completed explicit storage initialization
- **WHEN** Sam runs `docker compose up -d --build --wait` at the repo root
- **THEN** the command exits with status 0
- **AND** `curl -s http://localhost:5000/readyz` prints `{"status":"ready"}`

#### Scenario: The database holds the initial migration

- **WHEN** a first start has reached readiness
- **AND** a tester counts the rows of `_sqlx_migrations` where `success` is true, through `docker compose exec db psql`
- **THEN** the count is 2

#### Scenario: The app starts once on new volumes

- **GIVEN** explicit initialization completed on new volumes
- **WHEN** the app service first reaches readiness
- **AND** a tester reads the `app` container's `RestartCount` with `docker inspect`
- **THEN** the value is `0`

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

- **GIVEN** a ready instance
- **WHEN** Sam runs `docker compose down`
- **THEN** `docker volume ls` still lists the instance's database volume

### Requirement: The app has a writable storage volume

The `app` service SHALL mount a named volume at `/var/lib/sidereal/storage` by default.
The storage bind override SHALL mount the selected host directory at that same target.
Compose SHALL set `STORAGE_ROOT` to that target for serving and explicit initialization.
The app user SHALL be able to create, sync, and delete files there.
The server SHALL validate the marker and database pairing before serving HTTP.
Normal startup SHALL NOT create a marker, adopt storage, or initialize an empty substitute store.

#### Scenario: The app user can write to storage

- **GIVEN** a running initialized instance in either supported mount mode
- **WHEN** a tester creates and deletes a probe through `docker compose exec app`
- **THEN** the command exits with status 0

#### Scenario: Storage is a named volume

- **GIVEN** a running initialized instance without a storage bind override
- **WHEN** a tester inspects the app container's mounts
- **THEN** the mount at `/var/lib/sidereal/storage` has type `volume`

#### Scenario: A start leaves storage empty

- **GIVEN** an initialized store containing its marker and a disposable sentinel
- **WHEN** Sam recreates the containers and waits for readiness
- **THEN** marker and sentinel bytes are unchanged
- **AND** the directory contains no additional file

### Requirement: An installation guide tells the operator how to run the instance

The repo SHALL hold the installation guide `docs/install.md`. `README.md` SHALL link to it.
The guide SHALL cover these points:\

- the prerequisites: Docker Engine or Docker Desktop, with Compose 2.20 or newer;
- how to create `.env` from `.env.example`, with a command that generates a hex password;
- the commands that build the image, start PostgreSQL, explicitly initialize storage, and then start the app;
- the commands that stop and recreate the instance;
- that `down --volumes` deletes Docker-managed volumes but never deletes bind-mounted host data;
- that the app has no sign-in yet, so anyone who can reach its port can open it;
- that a server run outside the image needs a `WEB_ROOT` folder that only trusted users can write;
- that changing `POSTGRES_PASSWORD` after the first start does not change the database's password;
- how to retain mount selection and host paths in `.env`;
- how to prepare dedicated host directories and grant the app UID/GID `10001:10001` access;
- Desktop file sharing, rootless UID mapping, and host ACL or SELinux requirements;
- that PostgreSQL host data needs suitable local database storage;
- that operators must stop containers before remounting and verify the intended mount before initialization;
- that missing markers on paired stores require mount recovery, not reinitialization;
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

## ADDED Requirements

### Requirement: Operators can select persistent host directories independently

The repo SHALL provide `compose.storage-bind.yml` and `compose.database-bind.yml` beside `compose.yml`.
The storage override SHALL require an absolute `SIDEREAL_STORAGE_PATH`.
The database override SHALL require an absolute `SIDEREAL_DATABASE_PATH`.
The selected bind mounts SHALL use long syntax with `create_host_path: false`.
The operator SHALL select overrides through retained `COMPOSE_FILE` configuration or equivalent explicit `-f` arguments.
An invalid selected path SHALL fail rather than select a named volume or create a source directory.
Both overrides SHALL work independently and together without changing the other service's selected mount.
The app and initialization command SHALL retain UID/GID `10001:10001` and existing capability restrictions.

#### Scenario: Either directory can be chosen independently

- **WHEN** a tester resolves Compose with each override separately and then both together
- **THEN** each selected target has type `bind` and the configured source
- **AND** every unselected target remains a named volume

#### Scenario: A selected bind source is absent

- **GIVEN** a selected absolute path that does not exist
- **WHEN** Sam starts the affected service
- **THEN** Compose exits unsuccessfully and the source path still does not exist
- **AND** no fallback volume mounts at that service's selected target

#### Scenario: A selected bind path is empty or relative

- **WHEN** Sam selects a bind override with its path unset, empty, or relative
- **THEN** configuration validation or service startup exits unsuccessfully
- **AND** no app HTTP listener starts

### Requirement: Deployment verification demonstrates configuration and storage retention

`just instance-verify` SHALL demonstrate retention for default volumes and the combined bind-mount installation.
It SHALL compare database migration records, pairing identity, marker bytes, sentinel bytes, and retained settings across container recreation.
It SHALL demonstrate a changed storage host path without changing store identity.
The recipe SHALL use only its own disposable directories, Compose projects, and credentials.
It SHALL NOT initialize, modify, or delete operator-supplied data paths.

#### Scenario: Recreate both installations

- **WHEN** the verification recipe runs against default volumes and both bind overrides
- **THEN** it exits successfully only when all saved records and bytes remain identical after recreation
- **AND** the app reaches readiness without repeating initialization

#### Scenario: The shell holds operator mount settings

- **GIVEN** distinctive operator paths and `COMPOSE_FILE` in the caller's environment
- **WHEN** the verification recipe runs
- **THEN** those paths remain unchanged and no test container mounts them
