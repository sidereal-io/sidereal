## Why

Sam cannot tell a running app from a usable one. Docker and the home screen both check `/healthz`, so they report healthy while PostgreSQL is down.
When startup fails, one connection message covers an outage, a wrong password, and a TLS failure, so Sam cannot tell which to fix.

## What Changes

- The app image's health check calls `/readyz`. `docker compose ps` then reports the app unhealthy while its database is unusable.
- The home screen checks `/readyz` and shows a new `not ready` state when the server answers but its database is unusable.
- Startup splits the single connection failure into separate messages for an unreachable database, rejected credentials, a TLS failure, and a missing database. Each message names one fix and stays redacted.
- The server writes one log line when readiness becomes unusable, naming a safe failure category. It writes one more line when readiness recovers. Repeated failures write nothing new.
- The installation guide gains a troubleshooting section. It maps each startup message to its fix, explains the automatic restart, and warns that deleting volumes is never a recovery step.
- `just instance-verify` checks that the app reports unhealthy while PostgreSQL is stopped and healthy again after it restarts, with no data lost.

These features already exist, and this change keeps them: liveness separate from readiness, outage recovery, concurrent and repeated migrations, refusal of incompatible schemas without reset, transactional migrations, and redacted storage messages.
This change adds no health dashboard, admin screen, PostgreSQL upgrade path, backup tooling, second migration runner, or product schema.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `postgres-foundation`: Separate connection failure categories, and log lines for readiness changes.
- `compose-instance`: The container health check follows readiness, the guide gains a troubleshooting section, and verification covers an outage and its recovery.
- `web-shell`: The home screen checks readiness and shows `not ready` for an unusable database.

## Impact

The Rust changes affect `DatabaseError` and readiness in `core`'s database module, plus the existing server tests.
The `Dockerfile` health check, `docs/install.md`, and `scripts/instance-verify.sh` change.
In `web/`, `HealthStatus.tsx`, its tests, and the dev server's forwarded paths change. The change adds no design token.
The HTTP routes and their responses stay the same. No migration or schema change is needed.

Source story: https://github.com/sidereal-io/sidereal/issues/391.
