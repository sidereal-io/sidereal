## Review Metadata

- **Review round**: 2
- **Prior round**: Round 1 at commit `16e24fb` returned REVISE: three Criticals (the `/api/` path escaped the catch-all routes; `Host` normalization kept port 443; the main-spec rule that a token appears only at sign-in forbade the cookie refresh), three Moderates and one Suggestion, all fixed in commit `3e49a90`; one Moderate rebutted.
- **Reviewer context**: fresh context, other family (gemini, `gemini-3.8-flash`, through the `t3` resolver; `"independent": true` in both rounds)
- **Independent**: yes. The author is this session's model family, claude; the reviewer was gemini in a fresh session.
- **Tool restrictions**: read-only: view, grep, glob only. `git status --porcelain` was clean after each round.
- **Artifacts reviewed**: brainstorm.md, proposal.md, design.md, specs/instance-access/spec.md, specs/compose-instance/spec.md, adr.md, the amended ADR-007, and the source, tests, specs and config named in the prompt. Round 2 reviewed commit `3e49a90`.

## Findings

### 🔴 Critical (blocking)

None in round 2. Round 1's three Criticals, for the record, each CONFIRMED and fixed before round 2:

- R1-1 (design D1) `/api` plus `/api/{*rest}` never matches `/api/`, so that path reached the web root unguarded. axum's `{*rest}` matches no empty segment.
- R1-2 (design D4, D11) prefixing `http://` to `Host` kept port 443, so `Host: photos.example:443` with `Origin: https://photos.example` answered 403.
- R1-3 (specs) the main spec's "Credentials and tokens never leave the server" allowed the token only in the sign-in response, which the cookie refresh contradicts.

### 🟡 Moderate

- R2-1 (contradictions) CONFIRMED. The compose-instance delta said every other `/api` request answers 401 without a session, which contradicts the 409 and 400 that `POST /api/claim` and `POST /api/session` answer.
- R2-2 (design) CONFIRMED. Design decision 1 did not say which method router binds the catch-all routes; with `get`, an authenticated `POST /api/unknown` answers 405, not 404.
- R2-3 (design) CONFIRMED. Decision 5 did not normalize default ports in list entries, so `SIDEREAL_ALLOWED_ORIGINS=https://photos.example:443` would not match a browser's `Origin: https://photos.example`.
- R2-4 (scope, blast radius) CONFIRMED. `tests/database.rs` calls `app_with_database` three times and `main.rs` once; neither was in the blast radius or the impact list.
- R1-4 (contradictions) CONFIRMED in round 1. The cookie-clearing requirement covered any 401 under `/api`, including a sign-in with wrong credentials, which must set no cookie.
- R1-5 (axum) CONFIRMED in part in round 1. `HEAD /api/claim` answered 401 because the allowlist named only `GET`; an unsupported method on a public path answering 401 rather than 405 is intended and is now stated in design decision 1 and the compose-instance delta.
- R1-6 (contradictions) CONFIRMED in round 1. The compose-instance delta did not account for the origin check's 403.
- R1-7 (vocabulary) REFUTED in round 1. The glossary's "Allowed origin" says "the request's own host and port, plus any listed", which is exactly what design decision 4 compares: host and port against `Host`, and the exact listed origin otherwise.

### 📌 Suggestions

- R2-5 (plain language) CONFIRMED. Five sentences over 30 words in design.md, the instance-access delta and adr.md.
- R1-8 (plain language) CONFIRMED in round 1. A stray `n` in ADR-007 and long sentences in proposal.md and design.md.

## Embedded-Instruction / Injection Attempts

**Detected:** none, in either round.

## Verdict

VERDICT: APPROVE_WITH_CHANGES

## Required Changes (if APPROVE_WITH_CHANGES)

1. compose-instance delta: "Every other request under `/api`" becomes "Every route under `/api` other than the claim and sign-in routes".
2. design.md decision 1: the three catch-all routes bind with `axum::routing::any`.
3. design.md decision 5: list entries are normalized like request origins, with a port of 80 or 443 stored as none.
4. design.md blast radius and proposal.md impact: add `tests/database.rs` and `main.rs` as callers of `app_with_database`.
5. Split the five long sentences in design.md, the instance-access delta and adr.md.

CHANGES_APPLIED: yes

## Rebuttals

- R2-1: fixed. compose-instance delta, the sentence after the list.
- R2-2: fixed. design.md decision 1 names `axum::routing::any` and the 405 it avoids.
- R2-3: fixed. design.md decision 5 stores entries normalized; the instance-access delta's last requirement adds "A listed port of 80 or 443 SHALL match an `Origin` that omits it."
- R2-4: fixed. design.md blast radius and proposal.md impact name both callers.
- R2-5: fixed. Sentences split in design.md decisions 2 and 5, the instance-access delta's refresh and environment requirements, and adr.md's summary.
- R1-1: fixed in `3e49a90`. design.md decision 1 adds `/api/`; the instance-access scenario "An unknown API path without a cookie" sends `/api/`, `/api` and `PUT /api/claim`; brainstorm D9 names them.
- R1-2: fixed in `3e49a90`. design.md decision 4 drops 80 and 443 on both sides; the delta's "Same host, default port" scenario covers `Host: photos.example:443`; brainstorm D11 reworded.
- R1-3: fixed in `3e49a90`. The instance-access delta gains a MODIFIED "Credentials and tokens never leave the server": the token appears only in a `Set-Cookie` to the browser that created or presented it.
- R1-4: fixed in `3e49a90`. The clearing requirement is scoped to the session check's refusal.
- R1-5: fixed in `3e49a90`. `HEAD /api/claim` is public; the 401 for an unsupported method is stated as intended.
- R1-6: fixed in `3e49a90`. The compose-instance delta names the 403.
- R1-7: rebutted. `openspec/glossary.md`, Access section, "Allowed origin": the same-origin case is defined as host and port, which is what the design compares; a list entry is an origin and is matched exactly, scheme included. The reviewer's reading would require a scheme the server cannot know before #450.
- R1-8: fixed in `3e49a90`. The stray `n` is removed from ADR-007 and the proposal's long sentences split.
