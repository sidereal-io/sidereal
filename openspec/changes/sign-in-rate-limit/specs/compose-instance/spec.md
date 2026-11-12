## MODIFIED Requirements

### Requirement: An installation guide tells the operator how to run the instance

The repo SHALL hold the installation guide `docs/install.md`. `README.md` SHALL link to it.
The guide SHALL cover these points:

- the prerequisites: Docker Engine or Docker Desktop, with Compose 2.20 or newer;
- how to create `.env` from `.env.example`, with a command that generates a hex password;
- the one command that builds and starts the instance, including first-start storage initialization;
- that Sam claims the instance right after the first start, by opening the web page or by running `siderealctl reset-admin-password` first, because the claim is open to the first visitor until then, and how to keep the first start private for the terminal route: set `SIDEREAL_BIND_ADDRESS=127.0.0.1` and keep any reverse-proxy route off until the administrator exists;
- how to put the instance behind a reverse proxy that terminates TLS, with a Caddy example;
- that the proxy's address, as the app sees it, must be listed in `SIDEREAL_TRUSTED_PROXIES`, and how to find that one address for a proxy on the same machine and for one elsewhere;
- that requests carrying forwarded headers from an unlisted address are refused;
- that `SIDEREAL_BIND_ADDRESS=127.0.0.1` keeps the plain-HTTP port off the network when the proxy runs on the same machine;
- that `SIDEREAL_ALLOWED_ORIGINS` is optional and needed only when a proxy rewrites the `Host` header;
- the fixed sign-in limits: per client address, at most 1 failed sign-in per second, 5 per minute and 20 per hour; that a limited address is answered 429 with `Retry-After`, even for the correct password; that a successful sign-in and a restart clear the count; and that clients behind a proxy that sends no `X-Forwarded-For` share the proxy's address and so one limit;
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

#### Scenario: The guide states the sign-in limits

- **WHEN** a reviewer reads `docs/install.md`
- **THEN** it has a section whose heading contains `Sign-in limits`, placed after the reverse-proxy section, which names `429`, `Retry-After`, and the limits of 1 per second, 5 per minute and 20 per hour
