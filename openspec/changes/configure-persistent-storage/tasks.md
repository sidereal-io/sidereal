## 1. Extend database metadata

- [x] 1.1 Add `2_storage_identity.sql` with nullable `sidereal_metadata.storage_id UUID`; verify an upgrade preserves existing metadata values and migration 1's checksum and timestamp.
- [x] 1.2 Register schema baseline 2 and update migration-count and compatibility checks; run `deli -- just db-schema` and verify migration 1 and baselines 0 and 1 remain unchanged.

## 2. Implement managed-storage filesystem rules

- [x] 2.1 Add `core::storage` root validation and safe marker reading; verify existing-directory checks, the 4096-byte limit, unknown-field acceptance, and rejection of invalid fields, versions, JSON, symlinks, and special files without blocking.
- [x] 2.2 Add exclusive marker creation and file/directory sync; verify existing bytes are never overwritten, exclusive-create races re-read the root, and write or sync failures cannot authorize database pairing.
- [x] 2.3 Add uniquely named exclusive access probes that write, sync, and delete only their own files; verify successful cleanup, preserved existing files, and failure without ownership or permission changes.
- [x] 2.4 Add storage diagnostics with escaped container paths, failed operations, and safe recovery actions; verify missing-marker guidance directs mount recovery and output excludes marker bodies, database credentials, URLs, and raw database errors.

## 3. Pair storage during serialized startup

- [x] 3.1 Extend database initialization to pair storage after migrations and before releasing the existing startup lock, then run the access probe after committed or confirmed pairing; verify database-only initialization still works and read-only readiness never initializes storage.
- [x] 3.2 Implement parameterized pairing queries and all five design D2 rules; verify first installation, marker-only recovery, unchanged identities on restart, and refusal without file creation for missing, malformed, mismatched, or populated unpaired storage.
- [x] 3.3 Test interruption and concurrent pairing against PostgreSQL; verify same-root starts converge, different-root starts cannot replace a committed pairing, denied updates recover from marker-only state, and later probe failure preserves the committed UUID.
- [x] 3.4 Parse optional `STORAGE_ROOT` in the server and validate it with `WEB_ROOT` before connecting to PostgreSQL; verify unset storage preserves database-only health serving and invalid roots fail before database writes or HTTP binding.

## 4. Configure independent persistent mounts

- [x] 4.1 Set Compose `STORAGE_ROOT` and add independent storage/database bind overrides with required path variables and `create_host_path: false`; verify resolved mounts for defaults, each override, and both together, including relative-source resolution.
- [x] 4.2 Check invalid bind selections using disposable paths; verify empty settings and absent sources refuse without creating directories or using fallback volumes, while container targets, UID/GID, and capability restrictions remain correct.

## 5. Verify disposable deployments

- [x] 5.1 Isolate caller mount settings in `scripts/instance-verify.sh` and prepare narrowly mounted helper containers for test-directory ownership and cleanup; verify all bind paths belong to the recorded run under `.workspace/` and operator paths remain untouched.
- [x] 5.2 Extend recreation checks for default volumes and combined binds; verify identical `.env` bytes, migration records, database UUID, marker bytes, and sentinel bytes before and after recreation, retaining existing image, password, user, and capability checks.
- [x] 5.3 Add the paired-database/empty-storage refusal case; verify the diagnostic identifies the missing marker, no HTTP listener starts, no file is created, and the database pairing stays unchanged.

## 6. Document and check the installation

- [x] 6.1 Update `.env.example` and `docs/install.md` for independent mount selection, retained `COMPOSE_FILE`, platform separators, and explicit `-f` equivalents; verify the documented default command and mount combinations match resolved Compose configuration.
- [x] 6.2 Document dedicated subdirectories created only on mounted disks, UID/GID access, Desktop sharing, rootless mapping, ACL/SELinux needs, reliable sync, local PostgreSQL storage, and stopping before remounting; verify the guide explains host-to-container paths without recommending elevated app privileges or recursive permission repair.
- [x] 6.3 Document startup pairing, marker-only recovery, missing-marker refusal, mixed-mode `down --volumes`, permanent-loss recovery, simultaneous-loss limits, and rollback requirements; verify guidance preserves the one-command installation and distinguishes both retained data locations and `.env`.
- [x] 6.4 Run `deli -- just check` and `deli -- just instance-verify` with the required disposable fixtures; verify both pass and the resulting diff preserves plugin boundaries, existing migration files, and the agreed storage scope.
