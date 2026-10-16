## Why

Sam needs to choose disks for managed images and PostgreSQL without losing state when containers change.
The current named-volume deployment cannot distinguish a missing storage mount from an empty directory.

## What Changes

- Keep named volumes as defaults and add independent host-directory options for managed storage and PostgreSQL data.
- Add an explicit storage initialization command that pairs a durable storage marker with the database.
- **BREAKING**: Require initialization before the packaged instance serves HTTP, including installations using default volumes.
- Reject missing, mismatched, or unwritable managed storage without creating replacement storage or changing ownership.
- Preserve the pairing when operators move storage intact or change host paths.
- Document mount preparation, permissions, initialization, retained configuration, and recovery guidance.
- Extend local deployment verification to cover both mount modes, initialization failures, and retained state after container recreation.

This change adds no ingestion, object writes, watched folders, relocation tooling, or backup and restore tooling.

## Capabilities

### New Capabilities

- `managed-storage`: Explicit initialization, database pairing, startup validation, and safe storage errors.

### Modified Capabilities

- `compose-instance`: Configurable persistent locations, initialization before startup, retained state checks, and installation instructions.
- `postgres-foundation`: A forward migration for storage pairing and updated schema compatibility expectations.

## Impact

The implementation affects root Compose configuration, `.env.example`, `docs/install.md`, `Dockerfile`, and `scripts/instance-verify.sh`.
The Rust changes affect server configuration and command dispatch, plus generic storage and database modules in `core`.
The database gains a forward migration and schema baseline; existing migration files remain unchanged.
The storage identity contract warrants a durable ADR.
No HTTP mutation endpoint or web UI change is needed.

Source story: https://github.com/sidereal-io/sidereal/issues/390.
