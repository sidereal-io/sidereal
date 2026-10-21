## Why

Sam cannot tell a running app from a usable one. Docker checks `/healthz`, so `docker compose ps` reports the app healthy while PostgreSQL is down.
When startup fails, one connection message covers an outage, a wrong password, and a TLS failure, so Sam cannot tell which to fix.

## What Changes

- The app image's health check calls `/readyz`. `docker compose ps` then reports the app unhealthy while its database is unusable.
- Startup splits the single connection failure into separate messages for an unreachable database, rejected credentials, a TLS failure, and a missing database. A missing or unreadable certificate file gets its own configuration message. Each message names one fix and stays redacted.
- The server writes one log line when readiness becomes unusable, naming a safe failure category. It writes one more line when readiness recovers. Repeated failures write nothing new.
- The installation guide gains a troubleshooting section. It maps each startup message to its fix, explains the automatic restart, and warns that deleting volumes is never a recovery step.
- `just instance-verify` checks that the app reports unhealthy while PostgreSQL is stopped and healthy again after it restarts, with no data lost.
- A readiness probe that arrives while another check runs gets the last finished answer, not a fixed 503. Overlapping probes then stop seeing false outages.
- A connection lost during the startup or readiness checks of ownership and migration history is reported as a connection fault, not as damaged data.

The story's other needs already exist, and this change keeps them:

- liveness separate from readiness, and recovery after an outage;
- safe repeated and concurrent migrations, each in one transaction;
- refusal of incompatible schemas, with no reset or downgrade;
- redacted storage messages.

The web home screen keeps checking `/healthz` and stays unchanged. Sam's recovery path is the logs, `docker compose ps`, and the guide.
This change adds no health dashboard, admin screen, PostgreSQL upgrade path, backup tooling, second migration runner, or product schema.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `postgres-foundation`: Separate connection failure categories, a certificate file check, log lines for readiness changes, and the answer to an overlapping readiness probe.
- `compose-instance`: The container health check follows readiness, the guide gains a troubleshooting section, and verification covers an outage and its recovery.

## Impact

The Rust changes affect `DatabaseError` and readiness in `core`'s database module, the server's database configuration, and the existing server tests. `core` gains a direct `rustls` dependency with default features off.
The `Dockerfile` health check, `docs/install.md`, and `scripts/instance-verify.sh` change.
Nothing in `web/` changes, and the change adds no design token.
The HTTP routes and their responses stay the same. No migration or schema change is needed.

Source story: https://github.com/sidereal-io/sidereal/issues/391.
