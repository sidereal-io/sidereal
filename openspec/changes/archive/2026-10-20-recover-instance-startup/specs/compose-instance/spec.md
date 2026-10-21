## ADDED Requirements

### Requirement: The app's container health follows readiness

The app image's health check SHALL probe `/readyz`, not `/healthz`, every 10 seconds.
Docker SHALL report the `app` container `healthy` after a readiness check passes.
Docker SHALL report it `unhealthy` after three readiness checks in a row fail.
The health status is therefore a sample: Docker can take up to 40 seconds to report a new outage.
An unhealthy app SHALL keep running and keep answering `/healthz`. Docker SHALL NOT restart it because of its health.
The app SHALL report `healthy` again, without a restart, once its database is usable again.
The `db` service's health check SHALL stay unchanged.

#### Scenario: PostgreSQL stops after a ready start

- **GIVEN** a ready instance
- **WHEN** a tester runs `docker compose stop db` and waits 60 seconds
- **THEN** `docker inspect` reports the `app` container's health status as `unhealthy`
- **AND** `curl -s http://localhost:5000/healthz` prints `{"status":"ok"}`
- **AND** `curl -s http://localhost:5000/readyz` prints `{"status":"not_ready"}`

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
It SHALL list each startup and readiness message category with its likely cause and one recovery step.
The list SHALL cover missing or invalid configuration, unreadable certificate files, each connection cause, permission, PostgreSQL version, ownership, migration history, schema, deadlines, migration failure, and storage.
It SHALL explain that the app retries startup when Docker restarts it, so a database that starts late needs no action.
It SHALL tell the operator to restart with `docker compose up -d --wait` after fixing `.env`.
It SHALL state that deleting volumes or data directories is never a recovery step, and that it destroys the library.
It SHALL state that migration history from a newer release needs that release or a later one, never an older one.
It SHALL state that damaged or unexplained migration history needs investigation with the data kept as it is.
It SHALL state that editing migration records or the schema by hand to force a start is never a recovery step.

#### Scenario: The guide has a troubleshooting section

- **WHEN** a reviewer reads `docs/install.md`
- **THEN** it has a section whose heading contains `Troubleshoot`
- **AND** that section names `docker compose logs app` and `docker compose ps`

#### Scenario: The guide covers each connection cause

- **WHEN** a reviewer searches the troubleshooting section
- **THEN** it contains `database unreachable`, `database login rejected`, `database TLS verification failed`, and `database does not exist`

#### Scenario: The guide warns against resetting data

- **WHEN** a reviewer reads the troubleshooting section
- **THEN** it states that `docker compose down --volumes` is not a recovery step
- **AND** it states that editing migration records by hand is not a recovery step

### Requirement: Deployment verification demonstrates outage recovery

`just instance-verify` SHALL stop the `db` service of a ready default-volume instance.
It SHALL confirm that the `app` container becomes `unhealthy` while `/healthz` still answers `{"status":"ok"}`.
It SHALL start `db` again and confirm that the `app` container becomes `healthy` without a restart.
It SHALL confirm that the app's logs report the outage and the recovery.
It SHALL confirm that the successful migration records and the storage pairing are unchanged after recovery.

#### Scenario: Verify an outage and its recovery

- **WHEN** the verification recipe runs
- **THEN** it exits successfully only when the app turned `unhealthy`, then `healthy`, with an unchanged `RestartCount`
- **AND** the saved migration records and pairing identity match their values from before the outage
