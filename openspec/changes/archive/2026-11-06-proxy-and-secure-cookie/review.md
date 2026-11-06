## Review Metadata

- **Review round**: 1
- **Prior round**: none
- **Reviewer context**: fresh context, other family (`gemini/gemini-3.8-flash` through the `t3` resolver; the first-ranked `gemini/gemini-4-argon` answered only a quota error and was left out of the second run)
- **Independent**: yes. The plan was written in a Claude session; the reviewer is a Gemini model with no context from it.
- **Tool restrictions**: read-only: view, grep, glob only
- **Artifacts reviewed**: brainstorm.md, proposal.md, design.md, specs/, adr.md, the ADR-007 amendment of 2026-11-02, and the source files the prompt listed, at commit `d8f1c3c`
- **Effort**: high, from design.md's scrutiny tier

## Findings

### 🔴 Critical (blocking)

1. **Scheme and client address are unobservable at the `oneshot` seam** (test seams). Seven scenarios under "The scheme and the client address come from a trusted proxy" assert the resolved client address, and no response reflects it. CONFIRMED.
2. **Router layering leaves handlers without their extension** (contradictions, test seams). design.md applied `with_trusted_proxies` only in `main.rs`, while sign-in and sign-out took `Extension<Forwarded>`; `tests/access.rs:43` builds `app_with_database` alone, and `tests/database.rs`, `tests/healthz.rs` and `tests/web_root.rs` drive routers with no `ConnectInfo`, so either path answered 500. CONFIRMED.

### 🟡 Moderate

3. **CIDR matching** (trust boundary). Only the peer was canonicalised, so an IPv4-mapped item in the list or in `X-Forwarded-For` would not match; a prefix of 0 overflows a naive shift. CONFIRMED.
4. **`172.16.0.0/12` in the guide** (contradictions). design.md offered the Docker pool as the broad answer for a same-host proxy, which trusts every container on the host and contradicts the ADR-007 amendment. CONFIRMED.

### 📌 Suggestions

5. **Pass-through scenario covers one variable** (scenario testability). The Compose requirement names two settings; the scenario asserted one. CONFIRMED.
6. **Sentences over 30 words** (plain language). Two sentences in the resolver requirement and one guide bullet. CONFIRMED.

## Embedded-Instruction / Injection Attempts

**Detected:** none

## Verdict

VERDICT: APPROVE_WITH_CHANGES

## Required Changes (if APPROVE_WITH_CHANGES)

1. design.md: add a unit seam, `resolve(peer, headers, trusted) -> Forwarded` in `src/access/forwarded.rs`, for the seven resolver scenarios.
2. design.md: `with_trusted_proxies` is applied by `main.rs` and by the `tests/access.rs` helper; handlers take `Option<Extension<Forwarded>>` and read a missing extension as plain HTTP; a missing `ConnectInfo` is an untrusted peer at `0.0.0.0`, not a 500 and not loopback.
3. design.md Decision 4: canonicalise listed items, the peer and each `X-Forwarded-For` entry; a match needs the same family; the mask must not overflow at prefix 0; unit tests at 0, 1, 31, 32 and 0, 1, 127, 128.
4. design.md Risks: drop `172.16.0.0/12`; the guide tells Sam to list the one gateway address from `docker network inspect`.
5. specs/compose-instance: "A value passes through" asserts both `SIDEREAL_TRUSTED_PROXIES` and `SIDEREAL_ALLOWED_ORIGINS`.
6. specs/instance-access and specs/compose-instance: split the long sentences.

CHANGES_APPLIED: yes

## Rebuttals

1. Fixed: design.md "Test Seams" gains the unit seam; "Decisions" 1 names `src/access/forwarded.rs`.
2. Fixed: design.md Decisions 1 to 3 rewritten as the required change says; the matching risk line updated. The reviewer proposed a loopback default for a missing peer; the plan chose the unspecified address instead, because loopback may itself be a listed proxy and a default must trust nothing.
3. Fixed: design.md Decision 4.
4. Fixed: design.md Risks. `172.16.0.0/12` remains only in the spec's "A list" scenario, which tests parsing of a range, not advice to use one.
5. Fixed: specs/compose-instance "A value passes through".
6. Fixed: the resolver requirement is five short paragraphs; the guide bullet is four bullets.
