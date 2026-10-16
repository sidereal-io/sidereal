## Context

See `proposal.md` for motivation and scope.
`compose.yml` currently mounts two named volumes and starts the server after PostgreSQL becomes healthy.
The root `Dockerfile` creates writable storage for UID/GID `10001:10001`.
The server validates `WEB_ROOT`, initializes PostgreSQL through `core`, then binds HTTP.
The database foundation compares migration history and generated schema baselines during startup and readiness.

The accepted identity decision lives in `docs/decisions/ADR-014-managed-storage-identity.md`.
The user approved its architecture during exploration before this design was written.

## Goals / Non-Goals

**Goals:** Separate deliberate initialization from startup and preserve identity through container replacement and path changes.
Keep filesystem policy and database pairing generic within `core`.

**Non-Goals:** Detect live mount changes, prove image integrity, support multiple stores per database, or migrate an existing library.
Implement no HTTP setup endpoint, asset schema, object writes, or plugin behavior.

## Decisions

### D1. Use optional Compose override files for bind mounts

Keep `compose.yml` as the default named-volume installation.
Add `compose.storage-bind.yml` and `compose.database-bind.yml`, each replacing one service mount by its container target.
Use long bind syntax with `bind.create_host_path: false`.
Require absolute `SIDEREAL_STORAGE_PATH` and `SIDEREAL_DATABASE_PATH` values when their respective files are selected.
Retain the container targets `/var/lib/sidereal/storage` and `/var/lib/postgresql`.

Operators select files through `COMPOSE_FILE` in `.env`, so every later Compose command uses the same mounts.
The guide explains the platform-specific file separator and gives the equivalent explicit `-f` commands.
An unselected override ignores its path setting; selecting an override without its required setting fails.
Both bind options compose independently and together.

This avoids ambiguous strings that might select a named volume when the operator intended a directory.
Do not add an installer or generate Compose files.
Docker resolves paths on its daemon host; Desktop additionally requires host file sharing.
Operators mount network storage on that host before starting Compose.
Only managed image storage can use such a mount; PostgreSQL guidance requires suitable local database storage.
Managed storage must support exclusive file creation, file sync, directory sync, and reliable permissions.
Initialization reports unsupported operations instead of weakening durability checks.

### D2. Initialize through a local server command

Add `sidereal-server storage init`; the default command remains the HTTP server.
The documented first-install sequence builds the app, starts `db`, and runs:

```sh
docker compose run --rm --no-deps app sidereal-server storage init
docker compose up -d --wait
```

The command uses the app image, user, environment, and selected storage mount.
It opens no HTTP listener and needs no web root.
Only the explicit command can create pairing metadata or a marker.
It never runs automatically through `depends_on` or an entrypoint.
An unchanged rerun against a completed pair succeeds without changing either identity.

This is a host-authorized maintenance operation, not an unauthenticated network mutation mode.
Keep all current HTTP method restrictions and health response bodies.

### D3. Version the marker and reserve identity before writing it

Use a small UTF-8 JSON marker named `.sidereal-storage.json` at the storage root.
It holds exactly `format_version: 1` and a randomly generated UUID `storage_id`.
Accept only a regular file, reject symbolic links, and limit reads to 4096 bytes.
Reject malformed, unsupported, or extra fields rather than interpreting marker contents as paths.
The marker is metadata, not a secret or an integrity checksum.

Add migration `2_storage_pairing.sql` with a singleton pairing record containing a UUID and `pending` or `ready` state.
Leave the original migration and its generated schema baseline unchanged.
Add the second schema baseline and teach compatibility checks about the new prefix.
The migration creates an empty pairing table; it does not initialize storage.

Serialize initialization for one database with a dedicated PostgreSQL advisory lock and bounded acquisition.
The command validates the root and permissions before using the existing migration path.
It then follows these states while holding the pairing lock:

1. No pairing and no marker: require an otherwise empty root, then commit a new UUID in `pending` state.
2. Pending pairing and absent marker: require an otherwise empty root, then create the marker exclusively with that UUID.
3. Pending pairing and matching marker: sync the marker and root directory, then commit `ready`.
4. Ready pairing and matching marker: succeed without changing pairing or marker contents.
5. Any mismatch, malformed marker, unrelated contents during creation, or ready pairing without a marker: refuse without adoption or replacement.

The command syncs a newly written marker before committing `ready`.
An interruption before marker creation leaves a pending UUID that the explicit command can reuse.
An interruption after a complete marker leaves the same safe retry path.
A partial marker fails closed and requires diagnosis; no command overwrites it automatically.
If no pairing exists but a marker does, refuse adoption even when its format is valid.
Two databases racing for one empty root cannot overwrite each other's marker because creation is exclusive.
The losing database can remain pending and never serves HTTP.

This ordering avoids a ready database record referring to a marker that was never durably written.
Do not attempt a distributed transaction or invent a reset command.
The operator must verify the intended host mount before any explicit initialization or retry.

### D4. Validate storage before serving HTTP

Set `STORAGE_ROOT=/var/lib/sidereal/storage` in Compose; do not bake a host path into the server.
For direct server use, unset `STORAGE_ROOT` preserves the database-only health mode.
An explicitly empty or relative value fails configuration validation.

Normal packaged startup validates `WEB_ROOT`, then validates the storage root and marker, before database initialization.
The root must already exist as a directory; startup never creates it.
The final marker component cannot be a symlink; the operator controls root paths and ancestor symlinks.
Use exclusive, randomly named probe files to test create, write, sync, and delete access as the actual app user.
The probe never overwrites an existing file and normally leaves no file behind.
After database migrations, require a matching `ready` pairing before binding HTTP.
Startup never inserts or repairs pairing metadata.
A database newly created by PostgreSQL therefore cannot silently adopt an existing marked store.

An invalid local marker fails before application migrations.
A valid marker with a wrong or unpaired database can fail after structural migrations.
Do not promise that rejected pairing leaves an empty database structurally untouched.
Existing unrelated-database checks still run before migrations.

Keep filesystem parsing and validation in `core::storage` and pairing queries behind `core`'s database interface.
The `server` crate owns environment parsing and command dispatch.
The astro pack and public plugin ABI gain no dependency or contract.
Storage-enabled serving requires a pairing; direct database-only foundation tests remain possible.

### D5. Keep diagnostics actionable and bounded in disclosure

Define stable error categories for missing root, missing marker, invalid marker, mismatched pairing, incomplete initialization, and permission or sync failures.
Errors name `STORAGE_ROOT`, the failing operation, and the next safe action.
They advise checking the mount or app-user access before retrying.
Do not print configured path values, marker contents, credentials, database URLs, or raw nested errors.
Never recommend reinitialization for a ready store whose marker disappeared.

Keep UID/GID `10001:10001`, dropped capabilities, and `no-new-privileges` for the app and initialization command.
Operators prepare a dedicated host directory using targeted ownership or ACL grants.
Do not add root helpers, recursive ownership repairs, or broad permission grants.
PostgreSQL retains the pinned image's own user and initialization behavior; app UID rules do not apply to it.

### D6. Verify through the existing disposable deployment recipe

Extend `scripts/instance-verify.sh` using its existing project ownership checks and cleanup records.
Explicitly isolate `COMPOSE_FILE`, path settings, and `STORAGE_ROOT` from the caller's environment.
Use disposable bind directories under `.workspace/` and track them separately from Docker volumes.
Never delete an operator-supplied directory or recursively follow symlinks during cleanup.

Exercise default volumes and both bind overrides together; inspect the merged configuration for each independent override.
Initialize, save migration records and pairing identity, and create a disposable storage sentinel.
Recreate containers without initialization and compare every saved value, sentinel byte, and retained `.env` byte.
Stop containers and change the host path while retaining the marker to demonstrate path-independent identity.
Switch the same bind source to an empty directory or unrelated marked store and assert startup refusal.
Cover missing paths, app-user permission denial, and secret-bearing errors.
Retain the existing password, static serving, capability, and PostgreSQL image-pin checks.

Rust tests cover marker parsing, exclusive creation, interruption states, pairing races, permissions, and preservation of existing migration history.
Run `deli -- just check` and `deli -- just instance-verify` during implementation.
No token changes: none.

## Risks / Trade-offs

- **Copied markers pass identity checks.** Treat identity as configuration safety; future integrity verification checks image contents.
- **Mounts can change after startup.** Require the operator to stop the instance before remounting; runtime storage monitoring is deferred.
- **Initialization can leave a pending pair or partial marker.** Keep normal startup closed and document explicit retry versus diagnosis.
- **A selected database directory can be empty after a mount loss.** PostgreSQL may initialize it, but the app refuses the missing pairing.
- **Bind mounts vary across hosts.** Document daemon paths, Desktop sharing, rootless mappings, ACLs, and SELinux policy without disabling host protection.
- **Filesystem sync can block.** Support filesystems with working sync semantics; do not promise a bounded recovery time for a hung network filesystem.

## Migration Plan

Existing default volumes contain no managed images from this implementation line.
Stop the app, retain `.env` and both volumes, build the new image, then run explicit initialization against the existing empty storage volume.
The new migration preserves the first migration's record and all existing metadata.
Unexpected files require diagnosis; initialization never adopts or removes them.
Changing a populated library's location remains outside this installation procedure.

Rollback uses the previous binary only with its compatible pre-upgrade database state.
An older binary must reject the new migration history; do not edit history or delete volumes to force a downgrade.
