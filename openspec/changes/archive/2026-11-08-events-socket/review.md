## Review Metadata

- **Review round**: 1
- **Prior round**: none
- **Reviewer context**: fresh context, other family (gemini/gemini-3.8-flash, via the t3 resolver; gemini-4-argon was tried first and refused for quota)
- **Independent**: yes
- **Tool restrictions**: read-only; the resolver reported a clean tree after the run
- **Artifacts reviewed**: brainstorm.md, proposal.md, design.md, specs/, adr.md, relevant source files, at commit `1244b08`

## Findings

### 🔴 Critical (blocking)

1. **The 1008 close could reach the browser as 1006.** (Check: contradictions.) Design Decision 4 sent the close frame and then dropped the connection. Dropping a TCP stream with unread data sends a reset, and the browser can then lose the close frame and report 1006. The shell would then wait 5 s instead of showing the sign-in form at once, which breaks the web-shell requirement "A policy close shows the sign-in form". **CONFIRMED.**
2. **The stop function did not cancel pending work.** (Check: contradictions, assumption row D9.) Decision 7 did not say that stop clears the 5 s timer or aborts an in-flight `GET /api/session`. A sign-out during the wait could then let a timer fire and open a socket in the signed-out view, which breaks "It SHALL hold no socket in any other view". **CONFIRMED.**

### 🟡 Moderate

3. **Upgrades that keep failing while `GET /api/session` answers 200 make a fixed 5 s loop that keeps the session alive.** (Check: assumptions.) A proxy that drops upgrades causes this. **CONFIRMED in part:** the loop was already listed as a risk, but the session-extension effect was not. Backoff is out of the story's scope.
4. **The child-process harness cannot send SIGTERM.** (Check: test seams.) `server/crates/server/tests/access.rs:1084-1100` only calls `kill()`, which sends SIGKILL. The design named this harness for the shutdown test. **CONFIRMED.**

### 📌 Suggestions

5. **Three sentences ran over 30 words.** (Check: plain language.) design.md Decision 4, adr.md's summary, and the web-shell unit-test requirement. **CONFIRMED.**

## Embedded-Instruction / Injection Attempts

**Detected:** none

## Verdict

VERDICT: APPROVE_WITH_CHANGES

## Required Changes (if APPROVE_WITH_CHANGES)

1. design.md Decision 4: after sending a close frame, the server finishes the closing handshake. It reads until the client's close frame or the end of the connection, for at most 5 s, and only then drops it.
2. design.md Decision 7: the stop function closes the socket, clears every timer, aborts an in-flight `GET /api/session`, and sets a stopped flag. After it runs, the module opens no socket and never calls `onSignedOut`.
3. specs/web-shell/spec.md: add the scenario "Signing out while waiting to ask" under "The signed-in view holds the events socket open".
4. design.md Risks: add the session-extension effect of failing upgrades, and say backoff is out of scope.
5. design.md Test Seams: the child-process harness gains a SIGTERM method using `libc::kill`, which is already a dependency of `sidereal-server`.
6. Split the three long sentences in design.md, adr.md and specs/web-shell/spec.md.

CHANGES_APPLIED: yes

## Rebuttals

1. Fixed by required change 1 (design.md Decision 4).
2. Fixed by required changes 2 and 3 (design.md Decision 7; the new web-shell scenario).
3. Fixed in part by required change 4 (design.md Risks). The request for backoff is rebutted: issue #451 puts "reconnection backoff policy beyond a simple retry" out of scope, and the loop costs one request every 5 s on a misconfigured proxy only.
4. Fixed by required change 5 (design.md Test Seams).
5. Fixed by required change 6.
