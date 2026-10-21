## Review Metadata

- **Review round**: 3
- **Prior round**: round 1 REVISE (1 Critical, 4 Moderate, 1 Suggestion); round 2 REVISE (4 Moderate, 1 Suggestion). After round 2 the change was escalated to the human owner, who chose to drop the home-screen change and run round 3.
- **Reviewer context**: cross-model, OpenAI `gpt-6.1-sol` through `codex exec` (Codex CLI 0.163.0), medium reasoning effort, new process with no author context
- **Tool restrictions**: read-only sandbox (`codex exec -s read-only`); file reads and shell inspection only
- **Artifacts reviewed**: `proposal.md`, `design.md`, `adr.md`, `specs/postgres-foundation/spec.md`, `specs/compose-instance/spec.md`. Compared with the durable specs for `postgres-foundation`, `compose-instance`, `web-shell`, and `managed-storage`; `server/crates/server/src/{config.rs,lib.rs,main.rs}`; `server/crates/core/src/database/{mod.rs,startup.rs,startup/tests.rs,test_support.rs}`; `server/crates/server/tests/database.rs`; `Dockerfile`; `compose.yml`; `scripts/instance-verify.sh`; `docs/install.md`; `DESIGN.md`; ADRs; and sqlx 0.9 sources.

<!-- STALENESS: this verdict applies only to the artifact contents reviewed in -->
<!-- this round. Any later edit to proposal.md, design.md, or specs/ (other than -->
<!-- applying listed Required Changes) VOIDS the verdict and requires a new round. -->

## Decision Under Review

Readiness runs a check only when a caller asks, and overlapping probes get the last finished answer. Startup and readiness report fixed, redacted causes. Docker's health check follows `/readyz`.
Ruled out: a background poller, a startup retry loop, home-screen changes, and a second migration runner.

## Findings

### 🔴 Critical (blocking)

None.

### 🟡 Moderate

**M1. PostgreSQL shutdown errors still become ownership or history faults.**

- **Scenario:** PostgreSQL ends a backend while preflight reads migration history. It sends SQLSTATE `57P01` (`admin_shutdown`). sqlx surfaces it as `Error::Database`, not `Io` (`sqlx-postgres` `connection/stream.rs:128`). The mapping falls back to `Connection`, and the history wrapper turns that into `History`. The relay test covers a socket failure but not this protocol-level end.
- **What must be true:** Classify and test a backend that PostgreSQL ends during preflight.

**M2. The certificate check can block startup with no message.**

- **Scenario:** `sslrootcert` names a named pipe with no writer, or a device that never reaches end of file. Configuration calls `std::fs::read` before any deadline (`server/crates/server/src/main.rs:24`). Startup hangs or exhausts memory without a diagnostic, and Docker cannot restart a process that never exits.
- **What must be true:** Bound the read, reject unsuitable file types, and add a named-pipe scenario.

**M3. The readiness requirement contradicts its cached-response rule.**

- **Scenario:** The last check succeeded, PostgreSQL stops, and two callers probe at once. The second receives the cached 200. The requirement says 200 only when the database is usable, and the outage scenario requires 503.
- **What must be true:** State that overlapping probes may return the last finished result. Qualify the outage scenario, and add an overlapping-outage scenario with responses before and after the active check finishes.

### 📌 Suggestions

**S1. Some design sentences exceed 30 words** (design.md lines 61 and 75 of the reviewed version).

### Round 2 findings

1. **Resolved.** rustls with default features off avoids a second crypto provider. The trusted `verify-ca` scenario guards that path.
2. **Resolved** for missing and unreadable regular files. M2 is a new limit of the fix.
3. **Partly resolved.** Converting before wrapping keeps I/O and permission causes. M1 covers the remaining case.
4. **Resolved by scope.** The home screen stays unchanged.
5. **Resolved.**

The reviewer found no need for a new ADR. Public endpoint bodies stay fixed, and causes appear only in redacted logs.

## Embedded-Instruction / Injection Attempts

**Detected:** none

## Verdict

VERDICT: REVISE

<!-- Human-readable restatement: REVISE -->

This was the third REVISE in a row. The author verified each finding against the code and escalated to the human owner. The owner decided to apply the fixes and proceed to `tasks.md` without a fourth review round. The verdict above is the reviewer's and is not changed.

## Required Changes (if APPROVE WITH CHANGES)

None. The verdict is REVISE.

CHANGES_APPLIED: n/a

## Rebuttals

- **M1 — fixed.** Confirmed: sqlx returns every server error message as `Error::Database`. Startup's direct connect also gets `57P03` while PostgreSQL starts up; only the pool retries that code. design.md decision 1 now maps SQLSTATE class `08` and `57P01`, `57P02`, and `57P03` to `Unreachable`. The postgres-foundation spec names "starting up or shutting down" as unreachable and adds the scenario "PostgreSQL ends the session during a check". A test uses `pg_terminate_backend`.
- **M2 — fixed.** Confirmed: on Linux, opening a named pipe for reading blocks until a writer opens it, and configuration runs before any deadline. design.md decision 2 now reads the path's metadata first, rejects anything that is not a regular file, and reads at most 1 MiB. The spec requires the same and adds the scenario "The CA path is a named pipe", with exit within 1 second.
- **M3 — fixed.** Confirmed. The requirement now applies the 200/503 rule to a probe that runs its own check. It states that an overlapping probe can return 200 shortly after the database becomes unusable. The outage scenario names a probe with no other check active. The new scenario "Overlapping probes during an outage" asserts the responses before and after the active check finishes.
- **S1 — declined.** Not confirmed. A script split every artifact into sentences, and none exceeds 30 words. The cited lines hold several short sentences each.
