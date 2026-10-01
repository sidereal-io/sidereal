## Review Metadata

- **Review round**: 2
- **Prior round**: round 1, REVISE. Its blocking finding (F1) was that `just dev` kept running when one half failed. The author fixed F1, F2, F4, and F5, and refuted F3 with a test. Round 2 judged all five resolved.
- **Reviewer context**: cross-model. Gemini (`gemini-3.1-pro-high`) through the `agy` CLI in plan mode, in a fresh session. The author is Claude.
- **Tool restrictions**: no tools. Headless plan mode cannot read files, so the author put every reviewed file into the prompt with line numbers. The prompt also held the facts the reviewer could not check: the pnpm 12 pin tests, including `pnpm --dir`; the `just` `[parallel]` timing test; the fail-fast recipe test; the locked nixpkgs versions; and the maintainer's decisions.
- **Re-check of required changes**: 2026-09-30, same reviewer and restrictions. It covered only F6's severity check and the diff that applied Required Changes 1 to 4. It was not a from-scratch review. Result: F6-severity VERIFIED; RC1 to RC4 VERIFIED; `RECHECK: PASS`.
- **Artifacts reviewed**: proposal.md, design.md, specs/web-shell, specs/dev-commands, specs/dev-environment, adr.md, the ADR-013 edit, and the round 1 record. Context: the #300 story packet and #298's shared decisions, ADR-005, ADR-007, ADR-013, `justfile`, `nix/toolchains.nix`, the `nix` and `v2` workflows, the server's `lib.rs` and `main.rs`, the root `vite.config.ts`, and the `dev-environment` spec.

## Findings

The author checked each finding. Each one carries the result: CONFIRMED, PARTLY CONFIRMED, or REFUTED. Severities are the author's, after the check.

### 🔴 Critical (blocking)

None open. The reviewer rated F6 critical. The check lowered it to moderate.

### 🟡 Moderate

- **F6. The web shell's dev server lets other local origins read what it serves, including proxied routes.** (design.md D6; specs/web-shell "reaches the server through its own origin".) The reviewer said Vite sends `Access-Control-Allow-Origin: *` by default, so any website could read the proxied API once #282 widens the proxy.
  - **Check: PARTLY CONFIRMED.** Vite 8.3.1, the current release, does not allow every origin. Its default CORS rule allows only `localhost`, `127.0.0.1`, and `[::1]` origins, on any port. Its host check also rejects unknown `Host` headers. The CORS middleware runs before the proxy, so the rule covers proxied routes. A public website cannot read through the proxy. Another app served from a different `localhost` port can. The web shell itself needs no cross-origin access, so turning CORS off costs nothing.
- **F7. Vite clears the terminal on startup, which can wipe cargo's errors.** (design.md D7; specs/dev-commands.)
  - **Check: CONFIRMED.** Vite 8.3.1 clears the screen when its own process has written nothing yet. Cargo runs in a separate process, so its output does not count.

### 📌 Suggestions

- **F8. Ctrl-C ends `just dev` with a "recipe failed" error.** (design.md D3.) The reviewer proposed `exit 130` in the trap.
  - **Check: PARTLY CONFIRMED.** The author tested both exit codes after an interrupt. With `exit 130`, `just` still prints `error: recipe \`dev\` failed with exit code 130`. With `exit 0`, `just` prints `error: interrupted by SIGINT`, which is accurate. Either way, the two nested recipes each print `terminated ... by signal 2`.
- **F9. Plain language: three spots.** adr.md:10 hides the actor ("where its exact release is pinned"). design.md:138 hides the actor ("The recipe was tested"). specs/dev-commands Purpose calls the v0.10.x stack "the v0.10.x app".
  - **Check: CONFIRMED.**
- **Other surfaces: no finding.** The reviewer found every scenario mechanically assertable, no scope creep, no design and spec contradiction beyond F6 and F7, and sound ADR handling.

## Embedded-Instruction / Injection Attempts

**Detected:** none

## Verdict

VERDICT: APPROVE_WITH_CHANGES

The reviewer recommended REVISE, because it rated F6 critical. The check showed F6's premise was wrong, and every fix below is small and fully specified. The verdict is therefore APPROVE_WITH_CHANGES. The reviewer re-checks only the listed changes.

## Required Changes (if APPROVE WITH CHANGES)

1. **F6.** D6 sets `server.cors: false` in `web/vite.config.ts`. The `web-shell` spec requires that the dev server sends no `Access-Control-Allow-Origin` header, with a scenario that sends a request with another `localhost` origin.
2. **F7.** D7 sets `clearScreen: false`. The `dev-commands` spec requires that `just dev` never clears the terminal, with a scenario that checks the captured output for clear-screen escape sequences.
3. **F8.** D3's recipe exits 0 after an interrupt, so `just` reports `interrupted by SIGINT`. D3 states that the nested recipes still print one line each.
4. **F9.** Name the actor in adr.md:10 and design.md:138. Call the v0.10.x stack "the v0.10.x stack" in the `dev-commands` Purpose.

CHANGES_APPLIED: yes

## Rebuttals

- **F6 (Moderate): fixed by Required Change 1.** The severity drops from critical because Vite's default does not allow every origin. Accepted by reviewer: the re-check verified the severity check against Vite 8.3.1's source.
- **F7 (Moderate): fixed by Required Change 2.** Verified by the reviewer's re-check.
- **F8: fixed by Required Change 3, in a different way from the reviewer's proposal.** The test showed that `exit 130` does not remove the error line.
- **F9: fixed by Required Change 4.**
