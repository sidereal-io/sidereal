## Review Metadata

- **Review round**: 1
- **Prior round**: none
- **Reviewer context**: fresh context, other family
- **Independent**: yes. The review ran through `lets delegate review run` (resolver `t3`) in a fresh session on `gemini/gemini-3.8-flash`, while this plan's author is the claude family. The wrapper reported the session as "stopped to ask" (exit 5), but the thread had finished its single turn with a complete verdict and no pending question, so its final message is the review. Saved as `.workspace/claim-and-sign-in-review-raw.md` from thread `0e0a2d2e-4be2-4378-af61-5bd3c4fb9ba3`.
- **Tool restrictions**: read-only: view, grep, glob only. `git status --porcelain` showed no change from the reviewer.
- **Artifacts reviewed**: brainstorm.md, proposal.md, design.md, specs/, adr.md, ADR-007, ADR-015, and the source files the prompt named, at commit `1a76726`.
- **Effort**: high, from design.md's scrutiny tier.

## Findings

### 🔴 Critical (blocking)

1. **Malformed sign-in body answers 401, the spec says 400.** Design decision 7 mapped every JSON rejection to `InvalidCredentials`, which is 401 on sign-in, while `instance-access` "Malformed body" requires 400. Checks: contradictions. **CONFIRMED.**
2. **A closed instance still hashes on every `POST /api/claim`.** Design decision 2 hashed before the single `INSERT`, so a public route with no rate limit spends Argon2 CPU per request after the claim closes. Checks: where it stops working, trust boundary. **CONFIRMED.**

### 🟡 Moderate

3. **The unclaimed-start warning test would hang.** The design named the `.output()` seam in `tests/database.rs`, which only works when the binary exits; a successful start serves forever. Checks: test seams. **CONFIRMED.** `tests/storage.rs` already spawns, waits for `/readyz` and kills on drop.
4. **Cookie parsing breaks on `; `-separated cookies.** Splitting on `;` leaves a leading space on every name after the first. Checks: specific failure. **CONFIRMED.**
5. **The access section stays blank when the first probe fails.** Neither the design's state machine nor the `web-shell` delta said what a 503 or a network error on `GET /api/claim` shows. Checks: where it stops working, the day a service is down. **CONFIRMED.**

### 📌 Suggestions

6. **Sentences over 30 words** in `instance-access` (credential rules, sign-in), `web-shell` (access state) and design decision 9. Checks: plain language. **CONFIRMED.**

## Embedded-Instruction / Injection Attempts

**Detected:** none

## Verdict

VERDICT: APPROVE_WITH_CHANGES

## Required Changes (if APPROVE_WITH_CHANGES)

1. Design decision 7: a `BadRequest` variant answers 400 for a malformed body on both routes and for a claim that breaks the length rules; `WrongCredentials` answers 401 for a failed sign-in. The sign-in requirement in `instance-access` names the 400 for a malformed body.
2. Design decision 2: the claim reads whether an administrator exists and answers 409 before hashing; the partial unique index stays as the tie-breaker for a concurrent first claim.
3. Design test seams: the start-warning test reuses the spawn, wait-for-ready and kill pattern of `tests/storage.rs` with stderr piped.
4. Design decision 6: trim each `Cookie` pair before splitting on the first `=`.
5. Design decision 9 and the `web-shell` delta: a failed probe shows `Something went wrong` and no form, with a scenario for a 503 on `GET /api/claim`, and the unit tests cover that state.
6. Split the four long sentences.

CHANGES_APPLIED: yes

All six were applied in commit `fdf7788`, "docs: apply plan review changes to claim-and-sign-in (#447)". `openspec validate claim-and-sign-in --strict` passes after the edits.

## Rebuttals

- Finding 1: fixed. design.md decision 7 now has `BadRequest` (400) and `WrongCredentials` (401); `specs/instance-access/spec.md` "Sign-in creates a session" states the 400 for a malformed body.
- Finding 2: fixed. design.md decision 2 reads the claim state before hashing and keeps the index for the race.
- Finding 3: fixed. design.md Test Seams names `tests/storage.rs` `ready()` with stderr piped.
- Finding 4: fixed. design.md decision 6 trims each pair.
- Finding 5: fixed. design.md decision 9 adds the `failed` state; `specs/web-shell/spec.md` adds "The server is unavailable" and extends the unit-test requirement.
- Finding 6: fixed. The named sentences in both spec deltas and design decision 9 are split.
