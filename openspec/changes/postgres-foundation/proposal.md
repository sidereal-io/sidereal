## Why

Sam needs durable database state before deployment packaging and administrator setup can use it. The server currently reports process health without connecting to a database.

## What Changes

- Connect the server to a dedicated PostgreSQL 18 database through environment configuration.
- Apply forward-only migrations during startup, with ownership and compatibility checks before any database writes.
- Serialize concurrent startup and preserve migration state across server and database restarts.
- Keep `/healthz` for liveness and add `/readyz` for database readiness.
- Report startup failures without exposing credentials or resetting existing state.
- Add root recipes, real database tests, and operator guidance for the retained-state demonstration.
- **BREAKING**: Running the server now requires database configuration and compatible PostgreSQL.

## Capabilities

### New Capabilities

- `postgres-foundation`: Dedicated database configuration, safe startup migrations, readiness, and retained-state demonstration.

### Modified Capabilities

- `dev-commands`: The fresh-clone development scenario requires a prepared PostgreSQL database and database configuration.

## Impact

Core owns domain-agnostic database checks and migrations. The server wires configuration, startup, and health routes.
The change adds sqlx dependencies, database test infrastructure, root recipes, and server documentation.
CI supplies PostgreSQL for the existing server gate.

## Non-goals

This change excludes application packaging, browser changes, authentication, library tables, plugin execution, and backup tooling.
The database fixture supports development and testing. Compose plus `.env` remains the planned product installation path.
Auth planning owns ADR-007 wording and security sequencing. This story exposes only health checks and performs migrations at startup.
