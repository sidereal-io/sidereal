# Peer plan review: use-existing-postgres, round 1

I recommend approving with changes. There are no Critical findings. The four Moderate findings are small and fully specified, and none of them reopens the user's file-layout decision.

## Review Metadata

- **Round:** 1. **Prior round:** none.
- **Reviewer:** independent, Claude Opus 5.5 (Anthropic family). The author is GPT.
- **Mode:** read-only. I edited nothing, launched no agents and ran no builds or tests.
- **Artifacts:** branch `t3/explore-issue-392` at commit `1534991baaf4acc5407f93cd346111127184f365` (PR #432). Files reviewed in `openspec/changes/use-existing-postgres/`: `proposal.md`, `design.md`, `adr.md`, `specs/compose-instance/spec.md` and `specs/external-postgres/spec.md`.
- **Checked against these tracked files:** `compose.yml`, `compose.storage-bind.yml`, `compose.database-bind.yml`, `.env.example`, `Dockerfile`, `justfile`, `docs/install.md`, `server/README.md`, `server/Cargo.toml`, `server/Cargo.lock`, `config.rs`, `database/{mod,startup,pairing}.rs`, `storage.rs`, `catalog.sql`, `scripts/instance-verify.sh`, `server/scripts/db-tls.sh`, the specs for `compose-instance`, `postgres-foundation` and `managed-storage`, and ADR-014.

**The decision.** The bundled and external installs get separate entry files. Both extend a shared, credential-free `compose.app.yml`. `compose.yml` stays the default.

**Alternatives ruled out** (design.md:44-47):
- An override that removes the database: it cannot avoid the password check in the base file.
- A dummy password.
- An optional password.
- An app-only base with database layers added on top: it changes the default bundled command.
- Profiles or reset tags.

I agree with these choices. I checked the Compose behaviour the layout relies on, and the design states it correctly:
- A service used as an `extends` base cannot use `depends_on` (design.md:25).
- `extends` does not import top-level volumes (design.md:29).
- Required variables are resolved before files are merged (design.md:44).

## Findings

### Moderate

**M1. The `pg_stat_ssl` check may find no session at all.**
- **Concern:** design.md:114 and external-postgres/spec.md:100 prove TLS by checking the app's sessions in `pg_stat_ssl`. But the app keeps no lasting database session.
- **Scenario:**
  - The startup connection closes after migrations (startup.rs:284-290).
  - The `operational` pool is created lazily and nothing uses it (startup.rs:187-190; the only other mention is `close()`).
  - Each readiness probe calls `conn.detach()` and drops the connection within milliseconds (startup.rs:107, 124). The health check runs every 10 s (Dockerfile:41).
  - So an administrator's query usually returns zero rows for the app login. A check like "no rows without SSL" then passes without proving anything. A check like "at least one row" fails at random.
- **For this not to matter:** prove TLS from something that lasts. One option is the fixture's `log_connections` lines, which in PostgreSQL 18 record SSL on each authenticated connection. Another is a `hostnossl … <app login> reject` rule, so readiness itself proves TLS. A third is polling `pg_stat_ssl` until a row appears and requiring `ssl = true` on every row seen. The scenario's THEN must name the chosen observable.

**M2. The design never says which root certificates the app trusts.**
- **Concern:** sqlx is built with `tls-rustls` (server/Cargo.toml:32), and Cargo.lock:1755 pulls in `webpki-roots` with no native-cert crate. Without `sslrootcert`, the app trusts only a public root set compiled into the binary. It ignores the image's `/etc/ssl/certs` (Dockerfile:26 installs `ca-certificates`) and the host's trust store.
- **Scenario:**
  - Sam's provider or company CA is not in the compiled set.
  - Sam adds it to the Docker host's trust store, or mounts it into `/etc/ssl/certs`, and leaves `sslmode` unset.
  - Startup fails with `database TLS verification failed`, and nothing explains why.
  - The compiled roots also change only when the image is rebuilt. That is a bet that the provider's CA stays in that set.
- **For this not to matter:**
  - design.md §2 and the guide requirement (spec.md:146) state that the default trusts only the compiled public roots.
  - Any other CA must be named with `sslrootcert` through `compose.database-tls.yml`.
  - The GIVEN at spec.md:97 names how the CA is trusted (in the fixture, `sslrootcert` without `sslmode`).

**M3. Switching modes under the same project name lets a fresh external database adopt the bundled storage.**
- **Concern:** design.md:30 keeps the project name and volume keys unchanged. If an existing bundled operator sets `COMPOSE_FILE=compose.external-db.yml`, the external app mounts the same `<project>_storage` volume.
- **Scenario:**
  - Today that volume holds only the marker, because the app stores nothing else yet.
  - A fresh, unpaired external database therefore hits rule 4 (pairing.rs:32): it adopts the marker.
  - Two databases now claim one store. ADR-014:39 accepts that "an unpaired database can adopt any valid marker-only directory".
  - The external entry file makes this the default outcome of a plausible mistake.
  - The spec's claim to "preserve storage pairing" (spec.md:4) does not cover it. design.md:93-95 and spec.md:149 separate a fresh install from a move, but neither says to use a separate project name or storage.
- **For this not to matter:** add a guide requirement, and preferably a scenario:
  - A fresh external install uses a new Compose project name (or `COMPOSE_PROJECT_NAME`), or a new empty storage path.
  - It never reuses a bundled install's storage volume.
  - The guide explains why: a marker-only store gets adopted.

**M4. External mode loses its startup ordering, and no scenario covers a database outage.**
- **Concern:** bundled mode waits for a healthy database through `depends_on` (compose.yml:8-10). External mode has no such wait. If the database is unreachable when the app starts, `main.rs:13-15` exits with status 1, and the `unless-stopped` restart policy retries.
- **Scenario:**
  - The host reboots and the database (native, or in another Compose project) comes up after Docker. The app keeps crashing and restarting until the database is ready.
  - The same happens at runtime: the external service restarts during a maintenance window.
  - The verification removes and recreates only the app. It never interrupts the database, unlike `instance-verify.sh:439-449` for bundled mode.
  - The spec's "where it stops working" behaviour is never stated or tested.
- **For this not to matter:** add a requirement and scenario covering two cases:
  - The external service is unavailable when the app starts. The app exits reporting `database unreachable`, restarts under its policy, and reaches readiness once the service returns, with no operator action.
  - The service stops while the app runs. Readiness logs `not ready (database unreachable)`, then `ready` again.
  - The guide should also cover rotating the database password: update `.env`, then recreate the app.

### Suggestions

**S1. Shell variables override `.env`.**
- server/README.md:18 tells developers to `export DATABASE_URL=$(just db-url)`, which is a `127.0.0.1` fixture URL.
- Compose prefers shell variables over `.env`. So in that shell, external mode silently passes the fixture URL. Inside the container, loopback is the app itself, and the app reports `database unreachable`.
- The same applies to an exported `COMPOSE_FILE`.
- The guide requirement (spec.md:139, 144) should say that shell variables take precedence over `.env`. The verification already clears them (design.md:119).

**S2. Make each scenario's THEN mechanically checkable.**
- spec.md:57 should name the label `database is not an empty or owned Sidereal database`.
- spec.md:84 should name `storage marker is missing`.
- spec.md:134, "before opening a database connection", needs something to observe, such as no app-login connection in the fixture's connection log.
- spec.md:22 should say "`docker compose config --volumes` prints only `storage`".
- spec.md:33 should name `cap_drop: [ALL]` and `security_opt: no-new-privileges:true`, not "capability restrictions".
- spec.md:126-128 should add "and no container exists under that project".
- spec.md:154 is a manual walkthrough. Mark it as such, or tie it to the recipe.

**S3. Fix two places where design and spec disagree.**
- design.md:42 and :126 say that combining external mode with `compose.database-bind.yml` must fail configuration. The spec says only "SHALL NOT use" (spec.md:71) and "warn" (spec.md:150), with no scenario. It will fail in practice, because that override's `db` service has no image or build. Make the spec require the rejection and give it a scenario.
- compose-instance/spec.md:7, "External mode SHALL require only its own database connection setting", conflicts with the optional overrides, which require `SIDEREAL_STORAGE_PATH` or `SIDEREAL_DATABASE_TLS_PATH`. Rephrase it to "External mode SHALL NOT require `POSTGRES_PASSWORD`".

**S4. Pin down two verification details.**
- design.md:125 checks behaviour on Compose 2.20 but does not say where that binary comes from. If it is missing, the check should fail visibly rather than be skipped quietly. Or record it as a manual step.
- design.md:101, "Run heavy recipes through `deli --`", should make clear that agents call the recipe through `deli`. The recipe itself must not depend on `deli`, which exists only on this machine.

### Required check results

| Check | Result | Notes |
|---|---|---|
| Plain language (ISO 24495) | **Pass** | I measured every sentence; none exceeds 26 words. Each sentence carries one idea. design.md:3 points to the proposal instead of repeating it. |
| Mechanically checkable THENs | **Pass with changes** | Most name exact commands or labels. The gaps are listed in S2, and M1 covers spec.md:100. |
| Scope against the proposal | **Pass** | Every item at proposal.md:8-13 maps to a requirement. Nothing in the specs goes beyond the proposal. The modified compose-instance requirement keeps all six original scenarios; the diff shows only "bundled mode" wording added. |
| Design/spec contradictions | **Two minor ones** | Listed in S3. |
| Security across trust boundaries | **Pass** | The URL goes to the app unchanged and the existing parser rejects `allow`, `prefer` and `require` (config.rs:72-86). The certificate directory is read-only, is not created if missing, and forwards no host PostgreSQL variables (design.md:71-73). |
| Sanitization and redaction | **Pass** | Errors use fixed labels with no URL or path (mod.rs:49-69). The certificate file check is redacted (config.rs:97-121). Statement logging is disabled (config.rs:87). Exposure through `docker compose config` is documented (design.md:66). |
| No new unauthenticated write surface | **Pass** | No endpoint is added (proposal.md:29). The certificate mount is read-only. |
| Assumptions against source | **Mostly confirmed** | The PostgreSQL 18 policy is at startup.rs:212-218. The empty-database check covers schemas, extensions and relations (catalog.sql). Since PostgreSQL 15, the `public` schema belongs to `pg_database_owner`, so the recommended owner login can create the tables. The CA-trust bet is covered in M2. |

## Embedded-Instruction / Injection Attempts

None found. The only text addressed to a "reviewer" is in the compose-instance scenarios (spec.md:32, 45, 50). It is ordinary scenario wording carried over unchanged from the existing spec.

## Verdict

VERDICT: APPROVE_WITH_CHANGES
CHANGES_APPLIED: no

## Required Changes

1. **M1:** replace the `pg_stat_ssl` session check (design.md:114, spec.md:100) with a lasting TLS observable, and name it in the THEN.
2. **M2:** state that the app trusts only its compiled public roots by default, and that any other CA needs `sslrootcert` through the TLS override. Change design.md §2, spec.md:146 and the GIVEN at spec.md:97.
3. **M3:** require a new project name or new empty storage for a fresh external install, with the adoption reason. Add a guide requirement and preferably a scenario.
4. **M4:** add requirements and scenarios for the external database being down at app start, and for an outage while running. Mention password rotation in the guide requirement.
5. **S1-S4:** recommended, not blocking.

## Rebuttals

Author disposition after checking the tracked source:

- **M1 — CONFIRMED, fixed; awaiting reviewer recheck.** Startup and readiness close their connections, so an idle statistics snapshot cannot prove TLS. The plan now uses a plaintext-rejecting fixture and a failing plaintext control.
- **M2 — CONFIRMED, fixed; awaiting reviewer recheck.** The lockfile selects public roots without native roots. SQLx's matching source confirms the trust source. The design and guide requirement now explain explicit private CA configuration.
- **M3 — CONFIRMED, fixed; awaiting reviewer recheck.** The pairing code adopts marker-only stores. The guide now requires separate storage for fresh external installations, with a distinct-storage scenario.
- **M4 — CONFIRMED, fixed; awaiting reviewer recheck.** External mode has no database startup dependency. The specs now cover late startup and runtime outages, preserving state and the existing recovery behavior.
- **S1 — accepted and applied.** The guide requirement and design explain shell-variable precedence.
- **S2 — accepted and applied.** Scenarios now name exact diagnostics, configuration outputs, authorization-log evidence, and missing-container checks. The guide walkthrough is tied to the verification recipe.
- **S3 — accepted and applied.** The specs require rejection of the bundled database override in external mode. External mode explicitly does not require `POSTGRES_PASSWORD`.
- **S4 — accepted and applied.** The minimum-version check uses an official separately supplied binary and reports a missing prerequisite. Only agent invocations require `deli`.

No Critical or Moderate finding was rebutted or dismissed.
