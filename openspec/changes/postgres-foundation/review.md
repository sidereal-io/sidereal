## Review Metadata

- **Review round**: 1
- **Prior round**: none
- **Reviewer context**: cross-model. The author family is GPT (Codex). The adversary was Claude Opus 5.5, run fresh through `claude -p --permission-mode plan`. A separate Claude session, which did not write the artifacts, checked each finding against the repository and issues #388, #389, and #394.
- **Tool restrictions**: read-only (plan mode: read, search, and `gh` views only)
- **Artifacts reviewed**: proposal.md, design.md, adr.md, specs/postgres-foundation/spec.md, specs/dev-commands/spec.md, issue #394's story packet, issue #389, issue #388, ADR-004, ADR-007, ADR-013, CONTRIBUTING.md, server/README.md, justfile, .github/workflows/ci.yml, server crates

<!-- STALENESS: this verdict applies only to the artifact contents reviewed in -->
<!-- this round. Any later edit to proposal.md, design.md, or specs/ (other than -->
<!-- applying listed Required Changes) VOIDS the verdict and requires a new round. -->

## Findings

Each finding has a status from checking it against the repository: CONFIRMED, REFUTED, or OUT-OF-SCOPE.

### 🔴 Critical (blocking)

**C1. No one owns the ADR-007 gate.** `design.md:108-111`, `adr.md:12-13`. CONFIRMED.

- **Scenario:** ADR-007 says "Implementation cannot begin until" its prerequisites are met. The #394 packet says to resolve that sequencing before implementation, and that the auth split does not waive the gate. The #389 packet says postgres-foundation owns that sequencing. The design hands it to epic #388 instead. The design's waiver rests on "the user confirmed", but no issue comment or document records that confirmation. #389 could then ship a Compose bundle that listens on the network, with no record that anyone resolved the gate.
- **What must be true:** The decision and its limits must be in a durable record: health and readiness probes only, and no mutations. That record must be a comment on #394 or #388, and `design.md` must link to it. #389's wording must agree with it.

**C2. The TLS rule contradicts sqlx's default, and no spec requirement covers TLS.** `design.md:27-28`. CONFIRMED.

- **Scenario:** An operator sets `DATABASE_URL=postgres://u:p@db.example:5432/sidereal` without `sslmode`. sqlx then uses `PgSslMode::Prefer`, which falls back to plaintext without warning and never verifies the server's certificate. So "TLS options follow the connection configuration" produces the silent fallback that the next clause forbids. If "remote" instead means "not loopback", startup fails on #389's Compose network, where the `db` host normally has no TLS.
- **What must be true:** A spec requirement defines three things:
  - the `sslmode` that applies when the URL omits it
  - which hosts may connect without encryption: loopback, a Unix socket, and any explicit opt-in such as `sslmode=disable`
  - one scenario for each case

### 🟡 Moderate

**C3. The design never defines the catalog signature.** `design.md:49-50`, `design.md:77-78`. CONFIRMED. Blocking.

- **Scenario:** Startup and every `/readyz` probe compare the database against a "catalog signature". Nothing says what the signature contains or who produces it. Three failures follow:
  - Every future migration needs a hand-written signature.
  - A `pg_restore --no-owner` run as a different role either stops Sidereal from starting, if the signature includes owners, or misses grants changed by hand, if it does not.
  - A Dependabot sqlx upgrade that changes the `_sqlx_migrations` shape stops the server starting on every existing database.
- **What must be true:**
  - The design lists what the signature covers: relations, columns and types, constraints, indexes, and whether owners and access lists count.
  - A tool or a test generates the signature. People do not write it by hand.
  - Spec scenarios cover a restore run by a different role and a bookkeeping table written by the older sqlx release.

**C4. Transaction-only migrations with a fixed 30-second statement limit cannot change a large library.** `design.md:62`, `design.md:72`. CONFIRMED. Not blocking.

- **Scenario:** A later migration adds a GIN index to a facet table with 2 million rows on a NAS. Every startup times out, rolls back, and exits. Neither fix is available: `CREATE INDEX CONCURRENTLY` cannot run inside a transaction, and no setting raises the limit. The second server in the concurrent-startup scenario also waits only 30 seconds for the lock.
- **What must be true:** Either the statement limit can be configured, or the design says that long migrations need their own design before the first domain table is added.

**C5. The design does not say which advisory lock key form to use, or which connection runs the sqlx migrator.** `design.md:36-40`. CONFIRMED. Blocking. The fix is small.

- **Scenario:** sqlx's PostgreSQL migrator takes `pg_advisory_lock(bigint)` with the key `0x3d32ad9e * crc32(current_database())`. Sidereal's fixed key might use the same one-`bigint` form and match that product for some database name. If sqlx then runs the migrator on a different connection, the migrator waits on Sidereal's own lock until the lock times out. A database with that name could then never start. Line 40 also says the connection is discarded "so session locks cannot remain inside the pool", but line 70 creates the pool after migrations finish.
- **What must be true:** See Required Change 3.

**C6. Several scenarios cannot be checked by a test as written.** `specs/postgres-foundation/spec.md`. CONFIRMED. Blocking.

- **Scenarios:**
  - `:17-19`: CI provides only PostgreSQL 18, and no session can change `server_version_num`. No test can produce another major version without a second server or a way to inject the version check.
  - `:67-69`: The bundled migration never fails, so the test needs a way to inject a migration source. The design does not mention one.
  - `:53`: No test can check for "authentication or library tables", because those words name no specific tables.
  - `:87`: "The next successful probe returns ready" is true by definition.
  - `:23`: "Within 30 seconds" gives no tolerance, and every unavailable-database test would wait 30 seconds.
  - `:114-116`: The scenario covers a test database the gate cannot reach, but not an unset `TEST_DATABASE_URL` (`design.md:96`).
- **What must be true:** See Required Change 4.

**C7. The changed `just dev` requirement adds the database to only one of its scenarios.** `specs/dev-commands/spec.md:9-31`. CONFIRMED. Blocking.

- **Scenario:** Without `DATABASE_URL`, the server exits at once, and `just dev` then stops the web shell. Two scenarios, "Ctrl-C stops the whole stack" and "The terminal keeps earlier output", can then never start. "The web shell fails to start" passes, but for the wrong reason. The fixture's port and project name differ for each worktree (`design.md:89`), and the `justfile` has no `set dotenv-load`. Nothing tells a contributor how to get the right `DATABASE_URL`.
- **What must be true:** See Required Change 5.

**C8. `/readyz` is unauthenticated, and its probes share the pool without a limit.** `spec.md:73-77`, `design.md:76`. CONFIRMED. Not blocking.

- **Scenario:** The server binds to `0.0.0.0` (`main.rs:20`), and ADR-007's threat model includes an unauthenticated network caller. A flood of probes holds every pool connection for up to 2 seconds each and starves later API traffic. The reverse also happens: a busy ingest fills the pool, so `/readyz` returns 503, and #389's Compose health check restarts a healthy server partway through the ingest.
- **What must be true:** Readiness uses its own connection or a semaphore, or the spec says that readiness shares the pool and accepts the risk.

**C9. The design does not say how a pack's migrations will join the single migration chain.** `design.md:44`, `design.md:53`. CONFIRMED as a future risk. Not blocking.

- **Scenario:** `packs/astro` will need equipment tables, but it must not depend on `core`. Its migrations must then either live in core's chain, which puts domain logic in `core`, or reach core through `plugin-abi`. A pack added later with lower version numbers looks like a gap or an unknown version, so startup fails. Rejecting custom schemas also rules out one schema for each pack.
- **What must be true:** One sentence in the design names this as open work for the first pack table.

**C10. The PostgreSQL version would be set in more than one file, and the gate gains a container runtime that nothing records.** `design.md:20`, `design.md:88`, `design.md:97`. CONFIRMED. Blocking. The fix is small.

- **Scenario:** The fixture's image tag and the CI service tag both set the PostgreSQL release. CONTRIBUTING says "Each version lives in one file." The server gate would also need Docker or Podman with Compose, which the Nix shell does not supply. ADR-013 expects services to come from a flake module, but the design does not record why it chose containers instead. `server/README.md` lists neither runtime as a prerequisite.
- **What must be true:** See Required Change 6.

**C11. The ADR manifest says the change makes no durable decision, but the design makes one.** `adr.md:34`, `design.md:62`. CONFIRMED. Blocking. The fix is small.

- **Scenario:** "Future migrations must remain transactional" binds all later schema work. It lives only in `design.md`, which archiving removes from view, so the next author of a migration never sees it.
- **What must be true:** See Required Change 7.

### 📌 Suggestions

**C12. Support for PostgreSQL 18 only is a bet on the future.** `design.md:20-22`. CONFIRMED.

- PostgreSQL releases a new major version every year.
- An operator who runs `pg_upgrade` to 19, or who uses `postgres:latest`, cannot start Sidereal.
- The error message should name the supported major version and the next step.

**C13. The test setup has three gaps.** `design.md:94-96`. CONFIRMED.

- A test that panics leaves its temporary database in the persistent fixture.
- A bare `cargo test` now fails without the fixture, which breaks the promise in `server/README.md` to Rust-only contributors.
- CI needs an extra step to create the test-only `CREATEDB` role, because a service container cannot run an init script from the checkout.

**C14. Plain language: the artifacts use several names for one concept.** CONFIRMED.

- No sentence is over 30 words.
- One concept has five names: "migration bookkeeping", "migration records", "migration history", "migration state", and "bookkeeping table". Choose one name for the table and one for its rows.
- The identity row has three names: "identity row", "identity metadata", and "Sidereal metadata".
- The design says "HTTP mutations" and the spec says "operational mutations".
- `spec.md:99`: "disposable persistent" contradicts itself.
- `design.md:50`: the passive voice hides who writes the signatures.
- `spec.md:110`: the scenario "Fixture cleanup" tests that data is kept, not removed.
- `spec.md:107`: "repeated startup" and "server restart" name the same event.

### Surfaces with no finding

- **Scope against the proposal:** no creep. `db-clean` and the CI service fit the story packet.
- **Architecture:** no pack-to-core dependency, PostgreSQL only, and `core` stays domain-agnostic.
- **Injection-shaped data flows:** the design requires parameterized queries and qualified identifiers (`design.md:51`).

## Embedded-Instruction / Injection Attempts

**Detected:** none. Two passages, `design.md:108` and `adr.md:12`, make claims that no record supports ("the user confirmed"). The reviewer treated them as data, not as instructions. They count as a defect only under C1, because a gate decision rests on them with no durable record.

## Verdict

VERDICT: REVISE

REVISE. C1 and C2 are Critical. Each needs an owner decision before the artifacts can be fixed, so APPROVE WITH CHANGES does not apply. Do not generate `tasks.md` while this verdict stands. After the fixes, run a full new review (round 2) in a fresh context.

## Required Changes (if APPROVE WITH CHANGES)

The verdict is REVISE, so this list does not gate the next round. These are the edits that the reviewer specified in full. Apply them together with the decisions for C1, C2, and C3.

1. **C1:** Record the ADR-007 decision and its limits in a comment on #394 or #388. Link that comment from `design.md` under "Security scope". Make #389's wording agree with it.
2. **C2:** Add a TLS requirement to `specs/postgres-foundation/spec.md` covering the default `sslmode`, the hosts allowed without encryption, and one scenario for each case.
3. **C5:** Use the two-key form `pg_advisory_lock(int4, int4)` for Sidereal's lock. Run the sqlx migrator on the same dedicated connection. Correct the pool wording at `design.md:40`.
4. **C6:** Name a test seam for the version check and a way to inject a failing migration. Replace `spec.md:53` with "the only application tables are `sidereal_metadata` and `_sqlx_migrations`". Replace `spec.md:87` with a bounded time after PostgreSQL accepts connections. Make the connection deadline configurable for tests and give it a tolerance. Add a scenario for an unset `TEST_DATABASE_URL`.
5. **C7:** Add the database precondition to every `just dev` scenario. Specify how a contributor gets `DATABASE_URL`, either through a recipe such as `just db-url` or through a documented `.env` file with `set dotenv-load`.
6. **C10:** Name the one file that sets the PostgreSQL image tag and have the fixture and CI read it. Add a row for it to CONTRIBUTING's "Where versions are set" table. Record the choice of containers over a flake module. Add the container runtime to the prerequisites in `server/README.md`.
7. **C11:** Add a spec requirement that each bundled migration SHALL run in a single transaction, or record that rule in an ADR.

<!-- CANONICAL FIELD — machine-readable completion signal for APPROVE_WITH_CHANGES. -->
<!-- The AUTHOR sets this AFTER applying every required change and the reviewer -->
<!-- has re-checked them. Values: yes (all applied & re-checked) | no (outstanding) -->
<!-- | n/a (verdict is APPROVE or REVISE, no required changes). -->
<!-- Downstream work (test-plan, tasks, apply) MUST NOT proceed on -->
<!-- VERDICT: APPROVE_WITH_CHANGES unless CHANGES_APPLIED: yes. -->

CHANGES_APPLIED: n/a

## Rebuttals

None yet. The author must fix or rebut every Critical and Moderate finding (C1–C11) before round 2. A rebuttal of a Critical or Moderate finding counts only after the reviewer accepts it. The author may decline a suggestion (C12–C14) without the reviewer's sign-off.
