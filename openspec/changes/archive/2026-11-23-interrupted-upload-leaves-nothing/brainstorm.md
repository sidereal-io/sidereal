**Why:** Sam must be able to trust the store with files that cannot be replaced, and #521 only writes in the safe order: nothing yet proves that order holds when the server stops mid-upload, or removes the staging files such a stop leaves in `tmp/`.

## Story

#523 interrupted-upload-leaves-nothing — stopping the server mid-upload leaves no half-made asset, and the next start cleans up. https://github.com/sidereal-io/sidereal/issues/523

## Decisions

| # | Question | Answer | Source |
|---|---|---|---|
| D1 | What is in scope? | Startup cleanup of `tmp/`, a log line, and tests that stop the process at each ingest step. Out: checking stored objects against their hashes (#371) and reclaiming unreferenced objects in `store/`. | Evidence: #523 body |
| D2 | What must an upload cut off before commit leave behind? | No Asset or Asset version row. An object published before the stop may stay in `store/`, because no row refers to it and objects are never reclaimed in this slice. | Evidence: openspec/specs/managed-storage/spec.md ("Database commit fails after publication") |
| D3 | How should the tests stop the server at each ingest step? | Core tests run ingest in a child process and kill it at each step through the existing `Hook` in `storage/objects.rs`; a server-binary test seeds `tmp/` and checks the cleanup and the log line. | User |
| D4 | Which steps do the core tests kill at, and what does each leave? | Every `Step`: `WriteStaging`, `SyncStaging` and `Publish` leave a staging file and no object; `SyncObject` and each `SyncDirectory` level leave the object and no staging file. Every step leaves no rows, and a restart's cleanup empties `tmp/`. | Evidence: server/crates/core/src/storage/objects.rs |
| D5 | How does a core test kill at a step? | The test runs its own test binary again as a child, with environment variables that name the step, the database and the storage root; the child's hook aborts the process at that step. | Assumption |
| D6 | What if another server runs on the same storage folder? | One running server per storage folder is the supported setup. Cleanup runs under the startup lock; an upload in progress on the other server fails with 503 and saves nothing. | User |
| D7 | Where in startup does cleanup run? | Inside `initialize_with_storage`, after pairing confirms the marker and before the lock is released, so before the server binds HTTP and reports ready. | User; Evidence: server/crates/core/src/database/startup.rs |
| D8 | What if `tmp/` does not exist? | Cleanup does nothing and does not create it: a newly paired root keeps only its marker until the first ingest. | Evidence: openspec/specs/managed-storage/spec.md ("Newly paired root") |
| D9 | What does cleanup do with unexpected entries in `tmp/`? | It removes every entry that is not a folder, never following a symbolic link. It leaves subfolders and names them in the log. If `tmp` itself is a symbolic link or not a folder, startup stops with a storage error that names `tmp`. | User |
| D10 | What if a leftover staging file cannot be removed? | Startup logs the problem and continues. | User |
| D11 | When does the cleanup log line appear? | Only when cleanup removed at least one file. It says how many. Nothing is logged when nothing was removed. | User |
| D12 | What is the log line's wording, and who prints it? | Core returns what cleanup did; the server binary prints one `eprintln!` line, such as `storage: removed 3 leftover staging files from tmp`. | Assumption |
| D13 | Where does the cleanup code live? | In core's storage module, through the open root folder with `O_NOFOLLOW`, like the object store. It is not astronomy, so it does not belong in a pack. | Evidence: AGENTS.md; server/crates/core/src/storage/objects.rs |
| D14 | Should the term Staging file be defined, and where? | Yes, in openspec/specs/managed-storage/glossary.md: a file in `tmp/` that holds an upload's bytes while ingest checks them, before it becomes a stored object; one left behind by a crash is removed at the next startup. | User |
| D15 | Where is the one-server-per-storage-folder rule written down? | In the managed-storage spec requirement for cleanup. The installation guide does not change. | Assumption |

## Changes from the story

- Added: if `tmp` is a symbolic link or not a folder, startup stops with a storage error that names `tmp` (D9). The packet did not say what cleanup does with an unsafe `tmp`.
- Added: cleanup leaves subfolders of `tmp/` and names them in the log (D9).
- Added: a staging file that cannot be removed is logged, and startup continues (D10).
- Narrowed: the log line appears only when at least one file was removed (D11). The packet asked for a line that says how many.
- Stated: one running server per storage folder is the supported setup (D6).

Confirmed by user: yes
