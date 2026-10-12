## Review Metadata

- **Review round**: 3
- **Prior round**: Round 2 returned REVISE with one Critical and three Moderate findings. Round 1 also returned REVISE. The human owner then decided the Critical finding and approved a third round.
- **Reviewer context**: cross-model. The author is Claude. The reviewer is GPT, run as a new `codex exec` process with high reasoning effort. The review is independent.
- **Tool restrictions**: read-only sandbox (`codex exec -s read-only`)
- **Artifacts reviewed**: proposal.md, design.md, specs/, adr.md, and the relevant source files

## Round 2 findings, re-checked in round 3

| # | Round 2 finding | Round 3 result |
|---|---|---|
| C1 | The startup scan did not enforce the symbolic link rule | Fixed. The owner decided that the server trusts the contents of `WEB_ROOT`, and the spec no longer promises to refuse links |
| M1 | `--wait-timeout` was not one deadline for the whole start | Fixed. The script measures the time itself |
| M2 | Two verification runs in one checkout shared one project | Fixed. A lock allows one run per checkout |
| M3 | "Compose v2 or newer" promised too much | Fixed. The floor is Compose 2.20, stated and not proven |
| S1 | No scenario covered a request for a folder | Fixed |

## Findings

### 🔴 Critical (blocking)

None.

### 🟡 Moderate

**M1. Two checkout paths can share a checksum.** Confirmed. `server/scripts/db-fixture.sh` uses `cksum`, which is a 32-bit value.

- **The scenario:** The reviewer found two checkout paths with the same checksum. Each has its own lock, yet both use one Compose project. One run's cleanup can delete the other's database volume.
- **What must be true:** The project name is stronger, or a run refuses a project that another checkout owns.

**M2. The file service's folder handling breaks the stated HTTP surface.** Confirmed against the `tower-http` documentation.

- **The scenario:** `GET /assets` redirects to `/assets/` by default. A file `assets/index.html` would make `/assets/` return 200. The spec allows a status below 400 only for files and health probes.
- **What must be true:** The design turns off folder redirects and folder index files, and serves `/` explicitly.

**M3. `pg_isready` does not prove that the application database is usable.** Confirmed against the PostgreSQL documentation.

- **The scenario:** `pg_isready` reports a server as available even when the named database is missing. The spec said the check covers the application database.
- **What must be true:** The requirement describes server availability only, and leaves the application database to `/readyz`.

### 📌 Suggestions

None.

## Embedded-Instruction / Injection Attempts

**Detected:** none

## Verdict

VERDICT: APPROVE_WITH_CHANGES

APPROVE WITH CHANGES.

## Required Changes (if APPROVE WITH CHANGES)

1. Replace or guard the checksum-based verification project name.
2. Reconcile folder redirects and folder index files with the HTTP requirements, and cover `/assets` without a trailing slash.
3. Align the database health requirement with what its probe verifies.

The author applied all three in commit "docs: apply the round 3 required changes for compose-private-instance". The reviewer re-checked those three items in a new process and reported each as applied (`RECHECK: PASS`).

CHANGES_APPLIED: yes

## Rebuttals

- **M1 — fixed.** `design.md` D9 adds an ownership rule. The `dev-commands` delta adds a requirement and the scenario "Another checkout owns the project name".
- **M2 — fixed.** `design.md` D2 turns off the file service's folder handling and routes `/` explicitly. The `compose-instance` spec requires 404 for folder paths, and its scenario covers `/assets` and `/assets/` with an index file present.
- **M3 — fixed.** The `compose-instance` spec and `design.md` D6 now say the check covers TCP connections only.
- **Round 1, finding 7 (password alphabet) — rebutted, accepted by reviewer in round 2.** The password comes from the trusted operator, and a documented alphabet with redaction is reasonable in this scope.
