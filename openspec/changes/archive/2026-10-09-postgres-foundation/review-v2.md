## Review Metadata

- **Review round**: 2
- **Prior round**: REVISE (transport, schema checks, test seams, issue decision record)
- **Reviewer context**: cross-model Gemini 3.1 Pro (High) via agy CLI, high effort, plan mode, with --dangerously-skip-permissions explicitly requested by the user
- **Tool restrictions**: read-only: view, grep, glob only
- **Artifacts reviewed**: proposal.md, design.md, specs/postgres-foundation/spec.md, specs/dev-commands/spec.md, adr.md, .workspace/auth-boundary-confirmed.md, .workspace/issue-389-confirmed.md

<!-- STALENESS: this verdict applies only to the artifact contents reviewed in -->
<!-- this round. Any later edit to proposal.md, design.md, or specs/ (other than -->
<!-- applying listed Required Changes) VOIDS the verdict and requires a new round. -->

## Findings

### 🔴 Critical (blocking)

None.

### 🟡 Moderate

None.

### 📌 Suggestions

- **S1. Concurrent Readiness Flapping (Non-obvious failure mode):** The spec requires that readiness allows at most one active database check and that "excess probes SHALL return the fixed 503 response immediately." If a self-hoster runs a manual health check or a monitoring tool scrapes `/readyz` at the exact millisecond Docker Compose runs its health probe, one will receive a 503. Docker Compose healthchecks have retries (default 3), so this is mitigated for the container runtime, but returning a cached success for concurrent queries or allowing a tiny semaphore queue (e.g., 50ms) would prevent spurious "not ready" blips in logs.
- **S2. sqlx Atomic Bootstrap Implementation Risk:** The spec cleanly requires the initial transaction to atomically create both the migration table and the identity row. Because `sqlx::migrate!` internally attempts to create `_sqlx_migrations` in its own transaction before user migrations run, the implementer must ensure the custom `bootstrap` step executes a single transaction containing both `CREATE TABLE` statements before invoking the sqlx migrator, preventing sqlx from splitting the initialization. The design explicitly separates "bootstrap" and "sqlx migration execution," which perfectly supports this, so this is just a note for the task breakdown.

## Embedded-Instruction / Injection Attempts

**Detected:** none detected. The author submitted clean artifacts.

## Verdict

VERDICT: APPROVE

## Required Changes (if APPROVE WITH CHANGES)

CHANGES_APPLIED: n/a

## Rebuttals

All round-one findings have been addressed.

- **C1:** accepted by reviewer. Issue #394 comment explicitly confirms the auth boundary, and this is securely linked in `design.md`.
- **C2:** accepted by reviewer. Transport security modes, explicit opt-ins, and verified certificate scenarios are rigorously defined in `spec.md`.
- **C3:** accepted by reviewer. Restore scenarios and generated schema descriptions without physical identifiers resolve the check stability.
- **C4:** accepted by reviewer. Connection and migration timeouts are positively bounded and configurable.
- **C5:** accepted by reviewer. Two-int4 advisory lock and unpooled connection usage are cleanly specified.
- **C6:** accepted by reviewer. Private test helpers for injection and missing test config scenarios are present.
- **C7:** accepted by reviewer. `just dev` precondition added and root URL recipes are specified.
- **C8:** accepted by reviewer. Single-connection readiness pool and semaphore isolate health check capacity perfectly.
- **C9:** accepted by reviewer. Domain pack deferral is explicitly stated.
- **C10:** accepted by reviewer. `POSTGRES_IMAGE` pin is centralized in `server/postgres-image.env` and documented.
- **C11:** accepted by reviewer. Single transaction per migration and rejection of nontransactional sources are strictly required by the spec.
- **C12:** accepted by reviewer. Unsupported version errors specifically name PostgreSQL 18 and forbid data downgrade.
- **C13:** accepted by reviewer. Test setup uses a distinct test role, a supervisor connection for cleanup, and direct-cargo examples are appropriately updated.
- **C14:** accepted by reviewer. Plain language and nomenclature are consistently applied across all artifacts.

## Runner verification

The CLI reported that `--mode plan` had no effect while slash command expansion was disabled. The prompt required read-only review and stdout output. A before-and-after file hash comparison confirmed that Gemini changed no repository files. The runner saved this output as `review-v2.md`; it did not change the original review or planning artifacts.

- **S1 — partly refuted, otherwise noted.** `design.md:120` keeps Docker healthchecks on `/healthz`, which never queries PostgreSQL. Docker therefore does not compete for the readiness semaphore. Concurrent external `/readyz` probes can still return the specified 503 response. This remains a nonblocking suggestion.
- **S2 — confirmed as an implementation note.** `design.md:66–71` explicitly requires atomic bootstrap before invoking the default sqlx migrator. No planning change is required.
