## MODIFIED Requirements

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

### Requirement: An installation guide tells the operator how to run the instance

The repo SHALL hold the installation guide `docs/install.md`. `README.md` SHALL link to it.
The guide SHALL cover these points:

- the prerequisites: Docker Engine or Docker Desktop, with Compose 2.20 or newer;
- how to create `.env` from `.env.example`, with a command that generates a hex password;
- the one command that builds and starts the instance, including first-start storage initialization;
- that Sam claims the instance right after the first start, by opening the web page or by running `siderealctl reset-admin-password` first, because the claim is open to the first visitor until then;
- how to put the instance behind a reverse proxy that terminates TLS, with a Caddy example; that the proxy's address, as the app sees it, must be listed in `SIDEREAL_TRUSTED_PROXIES`, and how to find that address for a proxy on the same machine and for one elsewhere; that requests carrying forwarded headers from an unlisted address are refused; and that `SIDEREAL_BIND_ADDRESS=127.0.0.1` keeps the plain-HTTP port off the network when the proxy runs on the same machine;
- that `SIDEREAL_ALLOWED_ORIGINS` is optional and needed only when a proxy rewrites the `Host` header;
- the commands that stop and recreate the instance;
- that `down --volumes` with one bind override deletes only the named-volume data location;
- that retained database pairing with deleted storage refuses startup;
- that an unpaired database adopts retained marker-only storage, but refuses storage containing other files;
- that permanent paired-store loss requires restoring both locations together, or deliberately removing both locations to start fresh;
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
- **THEN** it has a section whose heading contains `Claim your instance`, and no section whose heading contains `no sign-in`

#### Scenario: The guide shows a reverse proxy

- **WHEN** a reviewer reads `docs/install.md`
- **THEN** it has a section whose heading contains `reverse proxy`, which holds a Caddyfile example and names `SIDEREAL_TRUSTED_PROXIES` and `SIDEREAL_BIND_ADDRESS`

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

### Requirement: Compose passes the access settings to the app

The `app` service SHALL pass `SIDEREAL_TRUSTED_PROXIES` and `SIDEREAL_ALLOWED_ORIGINS` from the Compose environment to the container, each unset in the container when it is unset in `.env` and the shell. Compose SHALL set no default for either. `.env.example` SHALL show both, commented out, with `SIDEREAL_BIND_ADDRESS`.

#### Scenario: Unset stays unset

- **WHEN** a tester runs `docker compose config` with neither variable set
- **THEN** the `app` service's `environment` holds neither `SIDEREAL_TRUSTED_PROXIES` nor `SIDEREAL_ALLOWED_ORIGINS` with a value

#### Scenario: A value passes through

- **GIVEN** a `.env` file that sets `SIDEREAL_TRUSTED_PROXIES=172.18.0.1`
- **WHEN** a tester runs `docker compose config`
- **THEN** the `app` service's `environment` holds `SIDEREAL_TRUSTED_PROXIES: 172.18.0.1`
