# managed-storage Specification

## Purpose

Sam can initialize managed storage during startup and detect missing storage using its retained PostgreSQL pairing.
The marker identifies the store independently of its host path.

## Requirements

### Requirement: Storage configuration is explicit

The server SHALL accept `STORAGE_ROOT` as an absolute path to an existing directory.
An explicitly empty or relative setting SHALL fail before database writes.
An unset setting SHALL preserve direct database-only health serving.
The server SHALL NOT create the storage root directory.

#### Scenario: Direct database-only operation

- **WHEN** a tester starts the server with valid database configuration and no `STORAGE_ROOT`
- **THEN** the server reaches readiness without accessing a managed storage directory

#### Scenario: Invalid storage configuration

- **WHEN** `STORAGE_ROOT` is empty, relative, absent from disk, or a regular file
- **THEN** startup exits unsuccessfully before database writes and creates no storage directory

### Requirement: Startup initializes only unpaired storage

Storage-enabled startup SHALL evaluate the database UUID and directory contents before serving HTTP.
An unpaired database with an empty root SHALL create and durably sync a marker before persisting its UUID.
An unpaired database with only a valid marker SHALL persist that marker's UUID after syncing the marker and root directory.
The server SHALL expose no separate storage initialization command or HTTP initialization endpoint.

#### Scenario: First startup

- **GIVEN** an empty writable storage root and an unpaired dedicated database
- **WHEN** Sam starts the instance
- **THEN** the app reaches readiness
- **AND** the marker and database contain the same UUID
- **AND** the root contains only the marker

#### Scenario: Interrupted first install leaves only a marker

- **GIVEN** an unpaired database and a root containing only a valid marker
- **WHEN** Sam starts the instance
- **THEN** startup persists that marker's UUID and reaches readiness without changing marker bytes

### Requirement: Startup refuses every other storage state and keeps both identities

A paired database SHALL require a valid marker carrying the same UUID.
Startup SHALL reject all other unpaired states and every paired state with a missing, malformed, or mismatched marker.
Startup SHALL NOT replace an existing marker or database UUID.
Repeated successful startup SHALL preserve both identities.

#### Scenario: Unpaired storage contains other files

- **GIVEN** an unpaired database and a root containing any file other than a valid marker
- **WHEN** Sam starts the instance
- **THEN** startup refuses HTTP, preserves existing files, and leaves the database UUID null

#### Scenario: Paired storage loses its marker

- **GIVEN** a paired database and an empty directory at the selected storage target
- **WHEN** Sam starts the instance
- **THEN** startup refuses HTTP and creates no file
- **AND** the database UUID remains unchanged

#### Scenario: Paired storage has another marker

- **GIVEN** a paired database and a valid marker carrying another UUID
- **WHEN** Sam starts the instance
- **THEN** startup refuses HTTP and preserves both existing identities

#### Scenario: Normal restart

- **GIVEN** a matching pairing and saved marker bytes and database UUID
- **WHEN** Sam recreates the containers
- **THEN** the app reaches readiness with the same marker bytes and UUID

### Requirement: Initialization remains safe across interruption and concurrent startup

The server SHALL sync the marker and parent directory before committing the database UUID.
Startup SHALL serialize pairing for one database and SHALL create markers exclusively without overwriting existing files.
Concurrent successful starts against one database SHALL observe the same UUID.

#### Scenario: Concurrent starts

- **WHEN** two servers start concurrently against one unpaired database and one empty storage root
- **THEN** every successful startup observes the same marker UUID and database UUID
- **AND** the storage root contains at most one marker

### Requirement: An interrupted pairing recovers or refuses

A failed or interrupted pairing SHALL leave the UUID null unless a concurrent successful startup committed it.
A subsequent startup SHALL recover a complete marker-only state without changing its identity.
A malformed partial marker SHALL cause refusal rather than automatic replacement.

#### Scenario: Marker write completes before database commit

- **GIVEN** a complete synced marker and a null database UUID after an interrupted startup
- **WHEN** startup runs again
- **THEN** it persists the same UUID and reaches readiness

#### Scenario: Marker write is incomplete

- **GIVEN** a truncated marker and an unpaired database
- **WHEN** startup runs again
- **THEN** it refuses HTTP and preserves the marker bytes and null UUID

### Requirement: Marker parsing permits compatible extensions

The marker SHALL be `.sidereal-storage.json` with required `format_version: 1` and UUID `storage_id` fields.
The server SHALL ignore unknown fields and reject unsupported versions, malformed JSON, or invalid required fields.
It SHALL accept only a regular non-symlink marker no larger than 4096 bytes.
Marker contents SHALL NOT select paths, commands, or database statements.

#### Scenario: Compatible additional field

- **GIVEN** a valid version 1 marker containing an unknown field and a matching database UUID
- **WHEN** startup validates the marker
- **THEN** startup accepts the marker without changing its bytes

#### Scenario: Invalid marker

- **WHEN** the marker is a symlink, directory, FIFO, socket, device file, oversized file, malformed JSON, or has unsupported or invalid required fields
- **THEN** startup refuses HTTP and leaves marker bytes and any symlink target unchanged

### Requirement: Storage access checks preserve operator data

The server SHALL validate create, write, sync, and delete access as the app user.
The server SHALL run the access probe only after pairing succeeds, while holding startup serialization.
Each probe SHALL use a unique name.
It SHALL use exclusive creation and remove its probe after successful validation.
Access failure SHALL prevent HTTP serving and leave existing storage files unchanged.
The server SHALL NOT change ownership, broaden permissions, or elevate privileges.

#### Scenario: Storage permission failure

- **GIVEN** storage that UID/GID `10001:10001` cannot write
- **WHEN** startup checks storage access
- **THEN** startup refuses HTTP and reports the failed operation
- **AND** storage ownership, permissions, and existing files remain unchanged

#### Scenario: Leftover probe

- **GIVEN** a paired store with a matching marker and a leftover probe file
- **WHEN** Sam starts the instance
- **THEN** the app reaches readiness and the leftover file remains unchanged

### Requirement: A refused pairing or failed probe keeps what exists

A pairing refusal SHALL create no file.
A later probe failure SHALL NOT clear the committed pairing UUID.

#### Scenario: A probe fails after pairing

- **GIVEN** a paired database, a matching marker, and a storage root where UID/GID `10001:10001` cannot create a file
- **WHEN** Sam starts the instance
- **THEN** startup refuses HTTP
- **AND** the database UUID remains unchanged

### Requirement: Storage diagnostics identify the failed path

Storage errors SHALL name the container-visible path, failing operation, and a safe recovery action.
The server SHALL escape control characters when rendering paths in logs.
It SHALL continue redacting database credentials, connection URLs, marker bodies, and raw nested database errors.
A paired database with no marker SHALL receive mount-recovery guidance rather than instructions to create a replacement marker.

#### Scenario: Missing paired marker

- **GIVEN** a paired database and a missing marker
- **WHEN** startup fails
- **THEN** server output includes the escaped storage path and mount-recovery guidance
- **AND** it contains neither the configured database password nor its connection URL

### Requirement: Ingest stages candidate bytes inside paired storage

Ingest SHALL copy candidate bytes into a unique `tmp/{uuid}` within the paired
storage root, compute their BLAKE3 hash and size, and have the Source recognize
the format. Staging and store SHALL be on the same filesystem. Startup SHALL
still leave a newly paired empty root with only its marker; ingest creates its
directories when first needed.

#### Scenario: Newly paired root
- **WHEN** the server starts with a newly paired empty storage root
- **THEN** the root holds only its marker until the first ingest creates `tmp` and `store`

### Requirement: Ingest publishes durable objects before library records

Ingest SHALL sync the staged file, atomically publish it at
`store/{h[0:2]}/{h[2:4]}/{h}` (`h` is the lowercase hex hash), and sync the
directory chain before committing any Asset or version. The chain is, in
order, `store/{h[0:2]}/{h[2:4]}`, `store/{h[0:2]}`, `store` and the root.
Each ingest SHALL sync the whole chain itself, whoever created it. A failed
write, sync or publication SHALL prevent the commit.

#### Scenario: First successful ingest
- **WHEN** Nova uploads a new recognized image into paired storage
- **THEN** its original exists at the BLAKE3-derived path with identical bytes before the Asset commit
- **AND** the file and every directory in the full chain have been synced before that commit

#### Scenario: Two hashes share a new prefix directory
- **WHEN** ingest A creates a prefix directory and has not yet synced it, and ingest B publishes a different hash under the same prefix
- **THEN** ingest B syncs that prefix directory and the rest of the full chain itself before its commit

#### Scenario: Publication fails
- **WHEN** a write, sync or atomic publication fails during ingest
- **THEN** no Asset or version for the upload is committed and existing stored objects remain unchanged

#### Scenario: Database commit fails after publication
- **WHEN** object publication succeeds but the Asset transaction fails
- **THEN** the library contains no partial Asset or version for that upload
- **AND** the published object remains available for later reuse and is not reclaimed

### Requirement: Matching content shares one stored object

When a regular object already exists at the candidate's hash path, ingest
SHALL reuse that object without overwriting it, once its bytes pass the check
below. Concurrent ingests of matching bytes SHALL retain one complete object
at that path. Stored objects SHALL NOT be reclaimed by this slice. What a
repeat upload does to the library is set in the asset-library capability.

#### Scenario: Matching bytes arrive again
- **WHEN** another accepted upload has the hash of an existing object whose bytes still match that hash
- **THEN** ingest reuses that object without changing it or creating another stored copy

#### Scenario: Concurrent matching bytes
- **WHEN** two requests concurrently publish the same candidate bytes
- **THEN** both publications use the candidate's hash path, and one complete stored object stays there
- **AND** neither request truncates or replaces the other's object

### Requirement: Ingest and reads stay inside managed storage

Object paths SHALL derive from core-validated hashes and staging paths from
core-generated identifiers, never from a display filename or caller path.
Storage operations SHALL reject path traversal, symlink escape and non-regular
object or staging files, including a symlink in an intermediate directory.
A refused operation SHALL NOT alter or disclose files outside managed storage.

#### Scenario: Filename contains traversal
- **WHEN** an upload supplies a name such as `../../outside.png`
- **THEN** no part of that name selects a storage location and no file outside the paired root is read or written

#### Scenario: Store directory is a symlink
- **WHEN** `store`, a hash directory or the final object is replaced by a symlink to a path outside the root
- **THEN** ingest or original retrieval refuses that path without reading or modifying the target

#### Scenario: Staging directory is unsafe
- **WHEN** `tmp` is a symlink or the proposed staging entry is already present or a special file
- **THEN** ingest does not follow or overwrite that entry and commits no Asset through it

### Requirement: An existing object is re-hashed before reuse

Before reusing an existing object, ingest SHALL compare the object's size
with the candidate's, and refuse without hashing when they differ. For a
same-size object, it SHALL compute the object's BLAKE3 hash, reading no more
than the candidate's size plus one byte. It SHALL reuse the object only when
its hash and size match the candidate's. Otherwise ingest SHALL refuse with a
storage failure, commit no Asset or version, and leave the object unchanged.

#### Scenario: Existing object does not match its hash
- **WHEN** an accepted upload's hash path holds a regular file of the same size whose bytes no longer hash to that path
- **THEN** the upload returns 503 and no Asset or version is committed
- **AND** the existing file's bytes, size and modification time are unchanged

#### Scenario: Existing object is much larger than the candidate
- **WHEN** an accepted upload's hash path holds a regular file far larger than the candidate
- **THEN** the upload returns 503 and no Asset or version is committed
- **AND** ingest reads none of that file's bytes, and the file is unchanged

### Requirement: Ingest uses the storage root that passed pairing

Startup SHALL resolve `STORAGE_ROOT` once, with symbolic links followed,
whether or not `WEB_ROOT` is set. Pairing, the web-root overlap check and the
object store SHALL all use that resolved folder. The object store SHALL open
the resolved folder once and keep that open folder for every ingest and
original read. Before it accepts work, it SHALL read the marker through that
open folder and refuse to start when the marker's UUID differs from the UUID
the database paired with.

#### Scenario: Storage root is a symbolic link
- **WHEN** `STORAGE_ROOT` is a symbolic link to a valid folder and `WEB_ROOT` is unset
- **THEN** the server starts, pairs that folder, and accepts an upload whose object appears under the link's target
- **AND** after a restart with the same settings, the pairing UUID is unchanged and the uploaded original is still returned

#### Scenario: Root replaced after pairing
- **WHEN** the folder at `STORAGE_ROOT` is replaced by another folder with a different marker after pairing succeeds and before the object store starts
- **THEN** startup fails before serving HTTP, with a message that names `STORAGE_ROOT`
- **AND** neither folder gains `tmp`, `store` or any object

### Requirement: Managed storage stays outside the web root

When both `STORAGE_ROOT` and `WEB_ROOT` are set, startup SHALL resolve both
paths with symbolic links followed. It SHALL refuse to start when they are the
same folder or one holds the other. It SHALL refuse before any database write
and before serving HTTP, with a message that names `STORAGE_ROOT` and
`WEB_ROOT`.

#### Scenario: Storage inside the web root
- **WHEN** `STORAGE_ROOT` is a folder inside `WEB_ROOT`
- **THEN** startup exits unsuccessfully before database writes, and its message names `STORAGE_ROOT` and `WEB_ROOT`

#### Scenario: Web root inside storage
- **WHEN** `WEB_ROOT` is a folder inside `STORAGE_ROOT`
- **THEN** startup exits unsuccessfully before database writes, and its message names `STORAGE_ROOT` and `WEB_ROOT`

#### Scenario: Same folder through a symbolic link
- **WHEN** `STORAGE_ROOT` is a symbolic link that resolves to `WEB_ROOT`
- **THEN** startup exits unsuccessfully before database writes, and its message names `STORAGE_ROOT` and `WEB_ROOT`

#### Scenario: Separate folders
- **WHEN** `STORAGE_ROOT` and `WEB_ROOT` resolve to folders where neither holds the other
- **THEN** startup continues to storage pairing as before

### Requirement: The server serves the web root it checked

Startup SHALL resolve `WEB_ROOT` once, with symbolic links followed. The
overlap check, the `index.html` check and the web-file fallback SHALL all use
that resolved folder, not the configured path. A `WEB_ROOT` symbolic link
repointed after startup SHALL change nothing that is served until the server
restarts.

#### Scenario: Web root link repointed after startup
- **GIVEN** `WEB_ROOT` is a symbolic link to a built web folder, and the server has started with a separate `STORAGE_ROOT`
- **WHEN** the link is repointed at the folder that holds `STORAGE_ROOT`, and a caller with no session requests a stored object's path through the web-file fallback
- **THEN** the response contains no bytes of the object
- **AND** `GET /` still returns the `index.html` of the folder the link named at startup

### Requirement: The installation guide covers what the overlap check cannot see

The startup overlap check compares resolved paths only. A symbolic link
created inside `WEB_ROOT`, and a bind mount or other mount that exposes the
storage folder inside `WEB_ROOT`, are outside it. The installation guide SHALL
state that the two folders must not overlap, and that no mount or link may
expose `STORAGE_ROOT` inside `WEB_ROOT`. It SHALL also say that the server
resolves `WEB_ROOT` at startup, so a repointed `WEB_ROOT` link takes effect
only after a restart.

#### Scenario: The installation guide warns about mounts and links
- **WHEN** a reviewer reads the section of `docs/install.md` on running the server outside the image
- **THEN** it says that `STORAGE_ROOT` and `WEB_ROOT` must not overlap
- **AND** it says that no mount or symbolic link may expose `STORAGE_ROOT` inside `WEB_ROOT`
- **AND** it says that a repointed `WEB_ROOT` link takes effect only after the server restarts

### Requirement: Startup removes leftover staging files

When storage is set, startup SHALL remove every entry in `tmp` that is not a
folder. Cleanup SHALL run after pairing confirms the marker and before the
startup lock is released, so before the server serves HTTP or reports ready.

#### Scenario: Leftover staging files are removed before readiness
- **GIVEN** a paired storage root whose `tmp` holds three regular files left by an earlier run
- **WHEN** Sam starts the server
- **THEN** `tmp` is empty when the server first reports ready
- **AND** the marker and every file under `store` are unchanged

#### Scenario: A special file in tmp is removed
- **GIVEN** a paired storage root whose `tmp` holds a FIFO with no writer
- **WHEN** Sam starts the server
- **THEN** startup does not block, the FIFO is gone from `tmp`, and the server reports ready

### Requirement: Cleanup never follows a symbolic link

Cleanup SHALL remove a symbolic link in `tmp` itself and SHALL NOT read,
change or remove the link's target.

#### Scenario: A symbolic link in tmp is removed without touching its target
- **GIVEN** a paired storage root whose `tmp` holds a symbolic link to a file outside the root
- **WHEN** Sam starts the server
- **THEN** the link is gone from `tmp`
- **AND** the target file still exists with the same bytes

### Requirement: Cleanup leaves subfolders of tmp

Cleanup SHALL leave each subfolder of `tmp` and everything in it.

#### Scenario: A subfolder of tmp is left in place
- **GIVEN** a paired storage root whose `tmp` holds a folder named `kept` that holds a file
- **WHEN** Sam starts the server
- **THEN** `tmp/kept` and the file inside it still exist, and the server reports ready

### Requirement: Cleanup changes nothing outside tmp

Cleanup SHALL NOT touch the marker, `store` or any other entry of the storage
root. When `tmp` does not exist, cleanup SHALL do nothing and SHALL NOT
create it.

#### Scenario: Newly paired root has no tmp
- **GIVEN** a paired storage root that holds only its marker
- **WHEN** Sam starts the server
- **THEN** the root still holds only its marker, with no `tmp` folder

### Requirement: One running server per storage folder

One running server per storage folder is the supported setup. When a second
server starts on a folder that another server is using, its cleanup can remove
a staging file that an upload in progress still needs. That upload SHALL then
fail with 503 and commit no Asset or Asset version.

#### Scenario: An upload in progress on another server sharing the folder
- **GIVEN** an upload whose bytes are staged in `tmp` and not yet published
- **WHEN** a second server's startup cleanup removes that staging file, and the upload then tries to publish
- **THEN** the upload fails with 503
- **AND** no Asset or Asset version for it is committed

### Requirement: Cleanup reports how many files it removed

When cleanup removed at least one file, the server SHALL print one line that
says how many, exactly `storage: removed <n> leftover staging files from tmp`,
with `file` in place of `files` when `<n>` is 1. When cleanup removed nothing,
the server SHALL print no count line. The lines for subfolders left and for
entries that could not be removed still apply.

#### Scenario: Count of removed files
- **GIVEN** a paired storage root whose `tmp` holds three regular files
- **WHEN** Sam starts the server
- **THEN** the server output contains the line `storage: removed 3 leftover staging files from tmp`

#### Scenario: One removed file
- **GIVEN** a paired storage root whose `tmp` holds one regular file
- **WHEN** Sam starts the server
- **THEN** the server output contains the line `storage: removed 1 leftover staging file from tmp`

#### Scenario: Nothing to remove
- **GIVEN** a paired storage root whose `tmp` is empty or absent
- **WHEN** Sam starts the server
- **THEN** the server output contains no line that starts with `storage: removed`

### Requirement: Cleanup names each subfolder it left

For each subfolder that cleanup left in `tmp`, the server SHALL print a line
that names the folder.

#### Scenario: A left subfolder is named
- **GIVEN** a paired storage root whose `tmp` holds a folder named `kept`
- **WHEN** Sam starts the server
- **THEN** the server output contains a line that names `"kept"` as a folder left in `tmp`

### Requirement: A file that cannot be removed does not stop startup

For each entry that cleanup could not remove, the server SHALL print a line
that names the entry and the reason, and startup SHALL continue.

#### Scenario: A file that cannot be removed
- **GIVEN** a paired storage root whose `tmp` holds a file, and a `tmp` folder from which the server's user cannot remove entries
- **WHEN** Sam starts the server
- **THEN** the server output names the file and the reason it could not be removed
- **AND** the server reports ready, and the file is still in `tmp`

### Requirement: Cleanup lines quote names safely

Each name in a cleanup line SHALL be quoted, with control characters escaped
as storage paths are, so that a name cannot add a line to the server output.

#### Scenario: A folder name holds a line break
- **GIVEN** a paired storage root whose `tmp` holds a folder named `kept` followed by a line break and `storage: forged`
- **WHEN** Sam starts the server
- **THEN** the server output names the folder on one line, as `"kept\nstorage: forged"`
- **AND** no output line starts with `storage: forged`

### Requirement: An unsafe tmp stops startup

When `tmp` exists and is a symbolic link or anything other than a folder,
startup SHALL stop before serving HTTP with a storage error that names `tmp`.
It SHALL NOT follow the link, and SHALL NOT change or remove `tmp` or its
target. The database pairing SHALL stay as it was.

#### Scenario: tmp is a symbolic link
- **GIVEN** a paired storage root where `tmp` is a symbolic link to a folder outside the root that holds a file
- **WHEN** Sam starts the server
- **THEN** startup exits unsuccessfully before serving HTTP, and its message names the tmp folder
- **AND** the link, the target folder and the file inside it are unchanged
- **AND** the database UUID is unchanged

#### Scenario: tmp is a regular file
- **GIVEN** a paired storage root where `tmp` is a regular file
- **WHEN** Sam starts the server
- **THEN** startup exits unsuccessfully before serving HTTP, and its message names the tmp folder
- **AND** the file `tmp` is unchanged

### Requirement: An ingest stopped before commit leaves no library record

When the server process stops at any filesystem step of ingest before the
Asset commit (writing or syncing the staging file, publishing the object,
syncing an existing object, or syncing any folder in the chain), the library
SHALL hold no Asset or Asset version for that upload. An object the upload
published before the stop SHALL stay in `store`, and this slice SHALL NOT
reclaim it, even when no record refers to it. The next startup SHALL leave
`tmp` empty of the upload's staging file.

#### Scenario: Stopped while the staging file is written or synced, or before publication
- **GIVEN** no object is stored for the upload's bytes, and no other upload stores them before the stop
- **WHEN** the server process is killed while it writes the staging file, while it syncs it, or just before it renames it into `store`
- **THEN** no Asset or Asset version for the upload exists, and no object exists at its hash path
- **AND** after the next startup, `tmp` holds no staging file

#### Scenario: Stopped after the object is published
- **WHEN** the server process is killed while it syncs any folder in the chain after publishing a new object
- **THEN** no Asset or Asset version for the upload exists, and the object exists at its hash path with the uploaded bytes
- **AND** after the next startup, `tmp` holds no staging file

#### Scenario: Stopped while reusing an existing object
- **GIVEN** an object already stored for the same bytes by an earlier upload
- **WHEN** the server process is killed while it syncs that existing object
- **THEN** the library holds the same Assets and Asset versions as before the upload, and the object is unchanged
- **AND** after the next startup, `tmp` holds no staging file
