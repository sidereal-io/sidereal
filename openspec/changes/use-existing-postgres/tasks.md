## 1. Compose installation modes

- [x] 1.1 Extract common app settings into `compose.app.yml`; verify bundled configuration preserves service names, resource names, build context, ports, security restrictions, and storage mounts.
- [x] 1.2 Add `compose.external-db.yml`; verify configuration lists only `app` and `storage`, requires `DATABASE_URL`, and succeeds without `POSTGRES_PASSWORD`.
- [x] 1.3 Add `compose.database-tls.yml`; verify its certificate mount is read-only, uses the documented container target, and refuses missing source paths without creating them.
- [x] 1.4 Verify both entry files with storage overrides; confirm external mode rejects `compose.database-bind.yml` and bundled mode still rejects missing or empty passwords.

## 2. Operator guidance

- [x] 2.1 Update `.env.example` with mode-specific settings and optional certificate paths; verify every supported setting appears without a real credential.
- [x] 2.2 Document entry selection, container reachability, URL encoding, shell precedence, TLS modes, compiled trust roots, and certificate access; verify examples match resolved Compose configuration.
- [x] 2.3 Document PostgreSQL 18, dedicated database ownership, migration privileges, metadata updates, and credential rotation; verify guidance matches existing startup and pairing operations.
- [x] 2.4 Document separate storage for fresh external installs, matching storage for established databases, and database lifecycle responsibilities; verify marker-adoption and recovery warnings are explicit.

## 3. Isolated external verification

- [x] 3.1 Add `scripts/external-db-verify.sh` and its root recipe with separate fixture/app projects, generated credentials, and cleanup ownership checks; verify caller settings cannot reach test resources.
- [x] 3.2 Add disposable fixture TLS and network setup using the existing image pin; verify app-only cleanup leaves the external service and fixture network running.
- [x] 3.3 Demonstrate named-volume and bind-storage initialization and app recreation; compare migration records, pairing, marker bytes, sentinel bytes, and external container identity before and after.
- [x] 3.4 Demonstrate fresh-install storage isolation and missing paired-marker refusal; verify distinct installation UUIDs, preserved existing storage, and unchanged pairing after refusal.
- [x] 3.5 Prove default remote TLS using plaintext rejection and a failing plaintext control; verify trusted `sslrootcert` configuration reaches readiness without explicit `sslmode`.
- [x] 3.6 Exercise untrusted CA and wrong-host cases while the fixture permits plaintext; verify startup refusal, exact diagnostics, unchanged database contents, and redacted app logs.
- [x] 3.7 Exercise unreadable certificate files, unrelated data, and incompatible history; verify the specified diagnostics, preserved state, and no authenticated connection for certificate-file failures.
- [ ] 3.8 Demonstrate late database startup and runtime outage recovery; verify startup retries, stable runtime restart count, expected health responses, recovery logs, and retained state.

## 4. Integrated validation

- [x] 4.1 Resolve the configuration matrix with official Compose 2.20 and the installed version; record versions and report unavailable minimum-version prerequisites as blocked.
- [ ] 4.2 Run `deli -- just instance-verify` and `deli -- just external-db-verify`; verify both demonstrations pass and leave only their intended diagnostic output.
- [x] 4.3 Run `deli -- just check` with its database fixture prepared; verify the server and web gates pass before the implementation PR proceeds.

## Validation evidence

- Configuration matrix passed on installed Docker Compose 5.6.0 and official Docker Compose 2.20.0 (release binary checked against its published SHA-256).
- Docker Engine for deployment verification: 29.9.2.
- Both versions preserve the original bundled configuration and resolve supported storage and TLS combinations. Missing credentials and unsupported external database-bind combinations are rejected.
- `deli -- just check` passed with this worktree’s database fixture prepared: server format, Clippy, tests and architecture lint; web type, lint, format, design, tokens and 84 tests.
