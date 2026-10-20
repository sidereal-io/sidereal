## Why

Sam needs to choose disks for managed images and PostgreSQL without losing state when containers change.
The current named-volume deployment cannot distinguish a missing storage mount from an empty directory.

## What Changes

- Keep named volumes as defaults and add independent host-directory options for managed storage and PostgreSQL data.
- Initialize storage during first startup, pairing a durable marker with the existing database metadata row.
- Recover an interrupted first installation when an unpaired database finds only a valid marker.
- Preserve the existing one-command Compose installation.
- Reject missing, mismatched, or unwritable managed storage without creating replacement storage or changing ownership.
- Preserve the pairing when operators move storage intact or change host paths.
- Document mount preparation, permissions, retained configuration, startup rules, and recovery guidance.
- Verify retained state after container recreation for default volumes and both binds, plus one missing-mount refusal.

This change adds no ingestion, object writes, watched folders, relocation tooling, or backup and restore tooling.

## Capabilities

### New Capabilities

- `managed-storage`: Startup initialization, database pairing, marker validation, and actionable storage errors.

### Modified Capabilities

- `compose-instance`: Configurable persistent locations, initialization during startup, retained state checks, and installation instructions.
- `postgres-foundation`: A forward migration for storage pairing and updated schema compatibility expectations.

## Impact

The implementation affects root Compose configuration, `.env.example`, `docs/install.md`, `Dockerfile`, and `scripts/instance-verify.sh`.
The Rust changes affect server configuration and startup wiring, plus generic storage and database modules in `core`.
The existing metadata row gains a nullable storage UUID through a forward migration and schema baseline.
Existing migration files remain unchanged; no pairing table is added.
The storage identity contract warrants a durable ADR.
No HTTP mutation endpoint or web UI change is needed.

Source story: https://github.com/sidereal-io/sidereal/issues/390.
