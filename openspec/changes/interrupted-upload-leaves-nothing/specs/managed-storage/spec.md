## ADDED Requirements

### Requirement: Startup removes leftover staging files

When storage is set, startup SHALL remove every entry in `tmp` that is not a
folder, after pairing confirms the marker and before the startup lock is
released, so before the server serves HTTP or reports ready. Cleanup SHALL
never follow a symbolic link: a link in `tmp` is removed itself, and its
target is not touched. Cleanup SHALL leave subfolders of `tmp` and everything
in them. When `tmp` does not exist, cleanup SHALL do nothing and SHALL NOT
create it. Cleanup SHALL NOT touch the marker, `store` or any other entry of
the storage root.

One running server per storage folder is the supported setup. When a second
server starts on a folder that another server is using, its cleanup can remove
a staging file that an upload in progress still needs. That upload SHALL then
fail with 503 and commit no Asset or Asset version.

#### Scenario: Leftover staging files are removed before readiness
- **GIVEN** a paired storage root whose `tmp` holds three regular files left by an earlier run
- **WHEN** Sam starts the server
- **THEN** `tmp` is empty when the server first reports ready
- **AND** the marker and every file under `store` are unchanged

#### Scenario: A symbolic link in tmp is removed without touching its target
- **GIVEN** a paired storage root whose `tmp` holds a symbolic link to a file outside the root
- **WHEN** Sam starts the server
- **THEN** the link is gone from `tmp`
- **AND** the target file still exists with the same bytes

#### Scenario: A special file in tmp is removed
- **GIVEN** a paired storage root whose `tmp` holds a FIFO with no writer
- **WHEN** Sam starts the server
- **THEN** startup does not block, the FIFO is gone from `tmp`, and the server reports ready

#### Scenario: A subfolder of tmp is left in place
- **GIVEN** a paired storage root whose `tmp` holds a folder named `kept` that holds a file
- **WHEN** Sam starts the server
- **THEN** `tmp/kept` and the file inside it still exist, and the server reports ready

#### Scenario: Newly paired root has no tmp
- **GIVEN** a paired storage root that holds only its marker
- **WHEN** Sam starts the server
- **THEN** the root still holds only its marker, with no `tmp` folder

#### Scenario: An upload in progress on another server sharing the folder
- **GIVEN** an upload whose bytes are staged in `tmp` and not yet published
- **WHEN** a second server's startup cleanup removes that staging file, and the upload then tries to publish
- **THEN** the upload fails with 503
- **AND** no Asset or Asset version for it is committed

### Requirement: Cleanup reports what it removed and what it left

When cleanup removed at least one file, the server SHALL print one line that
says how many, exactly `storage: removed <n> leftover staging files from tmp`,
with `file` in place of `files` when `<n>` is 1. When cleanup removed nothing,
the server SHALL print no line about cleanup. For each subfolder that cleanup
left, the server SHALL print a line that names the folder. For each entry that
cleanup could not remove, the server SHALL print a line that names the entry
and the reason, and startup SHALL continue. Names SHALL be quoted with control
characters escaped, as storage paths are.

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

#### Scenario: A left subfolder is named
- **GIVEN** a paired storage root whose `tmp` holds a folder named `kept`
- **WHEN** Sam starts the server
- **THEN** the server output contains a line that names `"kept"` as a folder left in `tmp`

#### Scenario: A file that cannot be removed
- **GIVEN** a paired storage root whose `tmp` holds a file, and a `tmp` folder from which the server's user cannot remove entries
- **WHEN** Sam starts the server
- **THEN** the server output names the file and the reason it could not be removed
- **AND** the server reports ready, and the file is still in `tmp`

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
SHALL hold no Asset or Asset version for that upload. An object published
before the stop SHALL stay in `store`, where no record refers to it, and this
slice SHALL NOT reclaim it. The next
startup SHALL leave `tmp` empty of the upload's staging file.

#### Scenario: Stopped while the staging file is written or synced, or before publication
- **WHEN** the server process is killed at the step that writes the staging file, syncs it, or publishes it
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
