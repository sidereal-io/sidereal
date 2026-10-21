## Why

Sam may already operate PostgreSQL and should be able to reuse that service for a private Sidereal instance.
The bundled installation now works, so an external connection can reuse its migrations, readiness checks, and storage safeguards.

## What Changes

- Keep `compose.yml` as the default bundled installation.
- Share app settings through `compose.app.yml` and add the app-only entry file `compose.external-db.yml`.
- Require `DATABASE_URL` only in external mode and `POSTGRES_PASSWORD` only in bundled mode.
- Retain the existing storage bind override and add an optional read-only database certificate mount.
- Document dedicated PostgreSQL 18 databases, role permissions, container networking, URL encoding, and verified TLS.
- Demonstrate external initialization, app recreation, retained state, and refusal of unsafe connections using disposable services.

## Capabilities

### New Capabilities

- `external-postgres`: Run the packaged app against a separate PostgreSQL service while preserving startup, transport, and storage safeguards.

### Modified Capabilities

- `compose-instance`: Scope the bundled password requirement to the bundled entry file while documenting all supported Compose settings.

## Impact

The implementation changes root Compose files, `.env.example`, installation guidance, and deployment verification scripts and recipes.
It reuses the existing app image, database configuration, migrations, readiness checks, and storage pairing.
It adds no database engine, migration, public endpoint, library dependency, or web UI behavior.
The default bundled command and existing persistent data remain compatible.

This change implements [story #392](https://github.com/sidereal-io/sidereal/issues/392).
