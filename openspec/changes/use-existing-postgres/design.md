## Context

See `proposal.md` for the operator outcome and `specs/external-postgres/spec.md` for the external installation contract.
The current `compose.yml` combines common app settings with bundled credentials and a database health dependency.
Both app and database settings require `POSTGRES_PASSWORD` during Compose interpolation.
The server already validates PostgreSQL 18, transport settings, schema compatibility, migrations, and storage pairing.
This change needs no new server startup path or database migration.

## Goals / Non-Goals

**Goals:** Keep each installation's required credentials separate while sharing app settings and preserving existing persistent resources.
Prove the external path using the packaged image and a database outside the app's Compose project.

**Non-Goals:** Provision remote services, widen PostgreSQL major-version support, migrate an established instance between services, or add connection pooling proxies.
The guide may explain prerequisites, but it will not automate database administration or provider-specific networking.
Web tokens: none.

## Decisions

### 1. Separate entry files share one credential-free app definition

Extract the existing app build, port mapping, storage setting, mount, capability restrictions, and restart policy into `compose.app.yml`.
Both entry files extend that service using `extends.file` and `extends.service`.
Keep every file at the repository root so build and mount paths resolve consistently.
The common definition contains no database URL, database dependency, or required database variable.

`compose.yml` retains the bundled connection, healthy-database dependency, database service, and database image pin.
`compose.external-db.yml` declares only `app`, sets its required `DATABASE_URL`, and declares the storage volume.
Each entry declares its own top-level resources because extending a service does not import those resources.
Keep project names, service names, volume keys, build context, and the image's UID/GID and health check unchanged.

Sam selects the external entry through retained `.env` configuration:

```dotenv
COMPOSE_FILE=compose.external-db.yml
COMPOSE_PROJECT_NAME=sidereal-external
DATABASE_URL='postgres://sidereal:encoded-password@postgres.example.net:5432/sidereal?sslmode=verify-full'
```

The normal `docker compose up -d --build --wait` command then starts only the app.
Sam can append `compose.storage-bind.yml` exactly as in bundled mode.
Document the Windows `;` separator and explicit `-f` alternatives.
Explain that exported shell variables override `.env`, including `DATABASE_URL` and `COMPOSE_FILE`.
The database bind override remains bundled-only; combining it with external mode must fail configuration validation.

An override that subtracts the bundled database cannot bypass required password interpolation in the original file.
Dummy passwords would confuse operators, while optional passwords would remove the bundled path's early validation.
An app-only base with additive database layers also works, but changes the default bundled invocation.
The selected layout preserves that invocation without introducing reset tags, profiles, or a newer Compose requirement.

### 2. Keep the server's PostgreSQL and transport policy

Document PostgreSQL 18 only and recommend its current minor update.
The single image pin stays in its existing source file and bundled copy; external servers need not match that image digest.
The server, rather than Compose, remains responsible for version and schema validation.

Pass `DATABASE_URL` directly to the app without rebuilding it from separate variables or adding `sslmode`.
Remote names default to `verify-full`; examples specify it explicitly so the intended protection is visible.
Literal loopback, `localhost`, and Unix sockets retain their existing local plaintext default.
Inside the app container, `localhost` refers to that container, not the Docker host or a sibling database.
The guide requires an address reachable from the app container and matching the database certificate.
The current SQLx rustls build trusts compiled public roots when no explicit CA is supplied.
It does not use the host trust store or certificates added to the image's `/etc/ssl/certs` directory.
Those compiled roots update only when the app image is rebuilt with updated dependencies.
Other CAs require an explicit `sslrootcert` path through the certificate mount described below.

Explain the accepted explicit modes: `disable`, `verify-ca`, and `verify-full`.
Explain that `verify-ca` checks certificate trust without hostname verification.
Only explicit `disable` permits unencrypted connections to a named service on a trusted network.
Reject `allow`, `prefer`, and `require` through the existing parser; never retry plaintext after certificate failure.
Document percent-encoding of URL credentials and single-quoted `.env` values when credentials contain dollar signs.
Keep connection URLs out of committed files and logs; warn that Compose inspection output can expose them.

### 3. Add an optional directory mount for certificate files

`compose.database-tls.yml` adds one long-syntax bind mount to the app.
Operator guidance selects it with the external entry, whose URL can name certificate files.
The bundled entry keeps its fixed local plaintext URL.
It requires `SIDEREAL_DATABASE_TLS_PATH`, mounts at `/etc/sidereal/postgres`, sets `read_only: true`, and sets `create_host_path: false`.
The source is a dedicated existing directory holding only the needed database CA, client certificate, and client key files.
The override sets no TLS mode and introduces no environment forwarding for host PostgreSQL variables.
Sam selects files through `sslrootcert`, `sslcert`, and `sslkey` using container-visible URL paths.

Mounting a directory supports a private CA alone or existing certificate-authentication settings without separate certificate-specific overrides.
The guide explains directory traversal access and file readability for UID/GID `10001:10001`.
Private keys stay restricted to the app's mapped identity; neither the app nor a helper changes operator permissions.
The existing parser checks named files before connecting and reports redacted failures.
An installation that needs no supplied certificate files does not load this override.

### 4. Recommend a dedicated login that owns its database and objects

Sam's administrator prepares a dedicated empty database and a non-superuser login before starting Sidereal.
Recommend that login own the database and the objects Sidereal creates.
Neither superuser nor `CREATEDB` is a runtime requirement.
Document `CONNECT`, public-schema `USAGE` and `CREATE`, and ownership needed for future migration DDL.
Existing objects need migration-table reads and pending-migration writes, plus metadata reads and updates for first storage pairing.
The owner recommendation covers these operations without presenting a read-only runtime role as sufficient.
Cluster access rules and network reachability remain administrator responsibilities.

The startup checks reject unrelated tables, extensions, custom schemas, and unsupported migration histories before changes.
The guide distinguishes a fresh external installation from moving an existing database.
An established database requires its matching managed-storage marker; selecting a new empty directory cannot reset that pairing.
Moving an instance requires consistent database and storage copies and remains outside this story's automation.
For a fresh external installation, choose a new Compose project name or a new empty managed-storage bind directory.
Never reuse another installation's named storage volume or bind directory for that fresh database.
An unpaired database can adopt a marker-only directory, so reusing it could pair two databases with one store.

External mode relies on the existing restart policy when its database starts late.
The guide explains startup retries and recovery without restarting an already running app after a database outage.
After rotating the password on PostgreSQL, Sam updates `DATABASE_URL` in `.env` and recreates the app to apply it.

### 5. Verify the packaged installation against a separate disposable service

Add `just external-db-verify` backed by `scripts/external-db-verify.sh`.
Keep the bundled `just instance-verify` checks as regression coverage after extraction.
Agents invoke heavy recipes through `deli --`; repository recipes do not require that machine-specific tool.

The external recipe owns separate fixture and app Compose projects, generated credentials, and a dedicated network.
Use the existing pinned PostgreSQL image from `server/postgres-image.env` for the fixture.
Generate fixture certificates under `.workspace/external-db-verify/` with a DNS name matching the fixture's network alias.
The existing development certificate only names `localhost`, so it cannot verify that network alias.
Use a separate wrong CA and a second network alias for rejection cases.
For the positive TLS case, reject plaintext for the app login with a `hostnossl` rule before broader access rules.
An explicit `sslmode=disable` control must fail, while the default remote mode reaches readiness with `sslrootcert` set.
Before certificate-rejection cases, reload access rules to accept plaintext and TLS so an accidental plaintext fallback would succeed and fail the test.
The fixture creates its dedicated database using an administrator; the app connects with a non-superuser owner login.

A test-only app override joins the fixture network without adding a database service to the product entry file.
Keep the network alive while removing the app project; remove it only during fixture cleanup.
Use a separate administrator connection to snapshot schema, rows, migration records, and storage pairing.
Use readiness against the plaintext-rejecting fixture and its failed plaintext control as stable proof of TLS.
Do not sample `pg_stat_ssl`: readiness closes connections immediately, so the view can be empty between probes.
Enable fixture connection authorization logging to prove unreadable certificate cases create no new authenticated app connection.
Use first-start and app-recreation cases for both named-volume and bind-mounted managed storage.
Check certificate mount permissions, startup refusal, unchanged data, and redacted logs for the spec's negative cases.
Compare the external service's container ID and start time around app removal and recreation.
Separately stop the fixture before app startup, observe a failed attempt, then start it and wait for automatic recovery.
After a ready start, interrupt the fixture and check readiness loss and recovery without app restart.
Save and compare migration records, pairing, and marker bytes across both cases.

The recipe clears inherited connection, Compose, PostgreSQL, certificate, and storage settings before generating its own environment file.
Always pass explicit project names, env files, project directory, and Compose file lists.
Never read the root `.env` or use caller-supplied connection URLs and data paths.
Follow the existing verification script's locking and resource-ownership checks; cleanup never uses broad container or volume pruning.
Capture command output privately under `.workspace/` and do not print resolved credential-bearing configuration.

Verify configuration resolution on Compose 2.20 and the installed current version.
For the minimum-version check, use a separately supplied official Compose 2.20 release binary and record its version.
Treat that check as a required manual compatibility check; report a missing binary as blocked rather than silently skipping it.
Check required-variable failures, app settings equivalence, supported override combinations, and invalid bundled-database override combinations.
Run deployment demonstrations on the available Docker installation and record its version.

## Risks / Trade-offs

- Shared-definition mistakes could change bundled mounts or image naming. Compare resolved configurations and rerun bundled retention checks.
- External services may install objects in template databases. Refuse unrelated objects and document a clean dedicated database requirement.
- Container addresses may differ from host addresses. Document container reachability and certificate-name matching without provider-specific recipes.
- Certificate directories expose their contents to the app. Require a dedicated directory and read-only access with narrowly granted permissions.
- App-only cleanup cannot manage remote backups. Explain responsibility and preserve the database/storage pairing during recovery.
- An existing bundled database may remain running after file selection changes. Do not describe mode selection as an in-place migration procedure.

## Migration Plan

Existing bundled operators keep their `.env`, project name, command, volumes, and bind overrides.
Fresh external operators select the alternative entry file and supply their dedicated database URL.
They use a separate project name or fresh managed-storage bind directory instead of another installation's storage.
The implementation introduces no database migration or data format change.
Reverting the Compose extraction preserves bundled resource names and stored data.
An external operator must keep the external configuration when reverting; starting a bundled database does not transfer external state.

## References

- [Compose interpolation](https://docs.docker.com/reference/compose-file/interpolation/) explains why required variables resolve before file merging.
- [Compose service extension](https://docs.docker.com/compose/how-tos/multiple-compose-files/extends/) documents shared settings and path resolution.
- [PostgreSQL version policy](https://www.postgresql.org/support/versioning/) recommends current minor updates within a supported major.
- [PostgreSQL schemas and privileges](https://www.postgresql.org/docs/18/ddl-schemas.html) explains database isolation and schema access.
- [SQLx transport modes](https://docs.rs/sqlx/0.9.0/sqlx/postgres/enum.PgSslMode.html) describes certificate and hostname verification.
