## Context

See `proposal.md` for motivation and scope.
`compose.yml` mounts two named volumes and starts the server after PostgreSQL becomes healthy.
The root `Dockerfile` creates writable storage for UID/GID `10001:10001`.
The database foundation serializes migrations and validates generated schema baselines before the server binds HTTP.

ADR-014 establishes startup initialization instead of an explicit initialization command.
The operator accepted the revised decision on 2026-10-19; independent plan review remains required.

## Goals / Non-Goals

**Goals:** Preserve the one-command installation and detect missing storage using the retained database pairing.
Keep generic storage policy within `core` and deployment configuration within Compose.

**Non-Goals:** Implement ingestion, object writes, relocation tooling, runtime mount monitoring, or multi-instance store ownership.
Add no HTTP mutation endpoint or separate initialization command.

## Decisions

### D1. Keep independent bind overrides

Keep named volumes in `compose.yml` as the default.
Add `compose.storage-bind.yml` and `compose.database-bind.yml` for independent host-directory selection.
Use long bind syntax with `bind.create_host_path: false` and required source variables.
The variables are `SIDEREAL_STORAGE_PATH` and `SIDEREAL_DATABASE_PATH`.
Keep container targets `/var/lib/sidereal/storage` and `/var/lib/postgresql`.

Retain selected files through `COMPOSE_FILE` in `.env`.
Document platform-specific separators and equivalent explicit `-f` arguments.
Recommend absolute host paths; Compose resolves relative paths against its project directory.
Do not pass host paths into the app or add application checks for their spelling.
Compose rejects missing bind sources instead of creating directories.
The marker check handles an existing empty directory beneath a missing managed-storage mount.

Host paths belong to the Docker daemon's machine; Desktop also requires file sharing.
Operators prepare mounts and access permissions before startup.
Managed storage requires reliable exclusive creation and file and directory sync.
Keep PostgreSQL on suitable local database storage.
Point each host path at a dedicated subdirectory below the disk's mount point.
Create that subdirectory only while the intended disk is mounted; never recreate it beneath an unmounted disk.
When that disk is unmounted, the source disappears and Compose refuses the affected service.

### D2. Initialize as part of existing startup

Retain `docker compose up -d --build --wait` as the installation command.
Set `STORAGE_ROOT=/var/lib/sidereal/storage` in Compose.
Unset `STORAGE_ROOT` preserves direct database-only health serving.
An explicitly empty or relative container path remains invalid.
The server never creates the storage root directory.

Validate `WEB_ROOT` and the local storage root before connecting to PostgreSQL.
Use the foundation's existing migration path, then evaluate these rules before binding HTTP:

1. Database paired and marker matches: continue, regardless of other storage contents.
2. Database paired and marker absent, malformed, or different: refuse without replacing either identity.
3. Database unpaired and root empty: create and sync a marker, then persist its UUID in the database.
4. Database unpaired and root contains only a valid marker: sync it and persist that same UUID.
5. Any other unpaired state: refuse without adopting files or changing the UUID.

Ordinary restart preserves both identities.
An interruption before marker creation leaves first-install conditions unchanged.
An interruption after a complete marker but before the database commit recovers through the marker-only rule.
Malformed partial markers fail closed; startup never overwrites them or deletes unknown files.
No setup command, force-rebind option, or pending/ready state is needed.

### D3. Extend existing metadata and startup serialization

Add nullable `storage_id UUID` to `sidereal_metadata` through `2_storage_identity.sql`.
A null UUID means unpaired; a non-null UUID means paired.
Do not create a separate pairing table or change the existing product identity and format version.
The migration leaves existing metadata unpaired until storage-enabled startup evaluates D2.

Keep migration 1 and its baseline unchanged.
Generate schema baseline 2 and update compatibility checks and migration-count assertions.
A column still changes the schema, so it requires a forward migration and new baseline.
Read-only database readiness validates the new schema without initializing storage.

Reuse the foundation's existing startup advisory lock through storage pairing, then release it.
Do not add another advisory lock or persisted initialization state.
The current code releases this lock immediately after migrations; storage setup must run before that release.
Keep database-only initialization available without storage setup.
Use the existing connection deadline for lock acquisition and parameterized pairing queries.
Preserve unrelated-database rejection before migrations.

This serializes simultaneous starts against the same database, including starts with different storage directories.
Publish markers with exclusive creation; never overwrite another marker.
If exclusive creation loses a race, re-read the root and apply D2 instead of adopting the losing UUID.
A partial write or failed sync leaves startup unsuccessful and the database unpaired.
Do not claim recovery from every possible interrupted file write.

The app role needs read access to the UUID and update access for first pairing.
A denied update leaves a marker-only state that can recover after the operator corrects permissions.

### D4. Keep the marker small and forward compatible

Use `.sidereal-storage.json` with required fields `format_version: 1` and UUID `storage_id`.
Ignore unknown fields; reject malformed JSON, invalid required fields, and unsupported versions.
Accept only a regular non-symlink marker and bound reads to 4096 bytes.
Open the marker without following symlinks and without blocking, then confirm the opened file is regular.
Marker data never becomes a filesystem path, command, or SQL fragment.
The marker is an identity check, not an integrity checksum or exclusive ownership claim.

### D5. Report useful paths without exposing database credentials

Storage errors name the container-visible `STORAGE_ROOT` path, failing operation, and a safe recovery action.
Render paths as escaped data so control characters cannot forge log lines.
Do not pass the daemon's host path into the app solely for diagnostics.
Explain the host-to-container mapping in the guide.
Continue redacting database credentials, connection URLs, marker bodies, and raw nested database errors.

Keep the app as UID/GID `10001:10001`, with dropped capabilities and `no-new-privileges`.
Validate create, write, sync, and delete access using an exclusively created disposable probe.
Run the probe only after pairing succeeds and the database UUID is committed or confirmed, while holding the startup lock.
Give each probe a unique name.
A pairing refusal creates no file.
Remove successful probes and preserve pre-existing files, including leftover probes in an already paired store.
A probe failure after successful pairing does not clear the committed UUID.
Grant access through targeted host ownership or ACLs; add no recursive repair or elevated app privileges.
PostgreSQL retains its image's own user and initialization behavior.

Filesystem logic belongs in `core::storage`; `server` owns environment parsing and startup wiring.
The astro pack and plugin ABI remain unchanged.
Network callers retain only static files and health probes, with no storage or pairing mutations through HTTP.

### D6. Keep deployment verification focused

Extend the existing disposable `scripts/instance-verify.sh` recipe.
Verify container recreation with default volumes and with both bind overrides together.
Compare retained `.env` bytes, migration records, database UUID, marker bytes, and a disposable storage sentinel.
Add one refusal case: a paired database with an empty directory at the managed-storage target.
The refusal must leave the pairing unchanged and create no file.
Check that its diagnostic names the missing marker, rather than an unrelated permission or database failure.

Keep the existing password, image-pin, app-user, and capability checks.
Isolate `COMPOSE_FILE`, path settings, and `STORAGE_ROOT` from the caller's environment.
Use only test-owned directories under `.workspace/` and preserve existing cleanup ownership checks.
Do not delete operator-provided paths or recursively follow symlinks.
Prepare storage bind directories for UID 10001 without requiring host root.
Use a disposable container for targeted test-directory ownership setup and removal of container-owned bind data.
Mount only this run's recorded, dedicated `.workspace` directories into that helper; never mount operator paths or the workspace root.
This helper belongs only to disposable verification, not the installation path or app startup.

Defer deployment tests for path moves, store swaps, and permission failures to ingestion work.
Keep focused Rust checks for startup rules, marker parsing, interrupted marker-only recovery, and concurrent pairing.
Run `deli -- just check` and `deli -- just instance-verify` during implementation.
Design tokens: none.

## Risks / Trade-offs

- **Both persistence locations disappear:** empty replacements look like first installation; document this limit and retain both locations together.
- **Unpaired database finds marker-only storage:** startup adopts it without proving its history; this is the deliberate recovery rule.
- **Partial marker write:** refuse and require diagnosis; do not silently delete or replace a malformed marker.
- **Another database uses the same marker:** the marker provides no ownership lease; support one instance per store.
- **Mount changes after startup:** operators stop containers before remounting; live monitoring is deferred.
- **Filesystem sync blocks:** require suitable storage and avoid promising a deadline for a hung network filesystem.
- **Mixed-mode deletion:** `down --volumes` removes named volumes but retains bind data, so it can delete only one side of a pairing.
  A retained database with lost storage refuses startup; a retained marker-only store can pair with a new database under D2.
  With additional storage files, a new unpaired database refuses adoption.
  Document restoring both locations from copies taken together, or deliberately deleting both locations to start fresh after permanent loss.
- **Host permission differences:** document Desktop sharing, rootless mappings, ACLs, and SELinux requirements without disabling host protections.

## Migration Plan

Existing instances keep `.env` and their volumes and use the same update command.
The forward migration adds a null storage UUID without changing existing metadata values or migration records.
Storage-enabled startup then initializes the existing empty volume under D2.
Unexpected files cause refusal rather than adoption or deletion.

Older binaries reject the new migration history.
Rollback requires compatible pre-upgrade database state; never rewrite history or delete retained volumes to force a downgrade.
