## Review Metadata

- **Review round**: 1
- **Prior round**: none
- **Reviewer context**: cross-model. Gemini (`gemini-3.1-pro-high`) through the `agy` CLI in plan mode, in a fresh session. The author is Claude.
- **Tool restrictions**: no tools. Headless plan mode cannot read files, so the author put every reviewed file into the prompt with line numbers. The prompt also held the facts the reviewer could not check: the pnpm 12 pin tests, the `just` `[parallel]` test, the locked nixpkgs versions, and the maintainer's decisions from the exploration.
- **Artifacts reviewed**: proposal.md, design.md, specs/web-shell, specs/dev-commands, specs/dev-environment, adr.md, and the ADR-013 edit. Context: the #300 story packet and #298's shared decisions, ADR-005, ADR-007, ADR-013, `justfile`, `nix/toolchains.nix`, the `nix` and `v2` workflows, the server's `lib.rs` and `main.rs`, the root `vite.config.ts`, and the `dev-environment` spec.

## Findings

The author checked each finding against the repo. Each one carries the result: CONFIRMED, REFUTED, or PARTLY CONFIRMED.

### 🔴 Critical (blocking)

- **F1. `just dev` keeps running when one of its two recipes fails.** (design.md D3; specs/dev-commands "One command runs the v2 stack".) If `web` fails, for example because `pnpm install` fails, `just` waits for `server`, which never ends. The reviewer proposed `concurrently --kill-others-on-fail` or a shell script with `wait -n`.
  - **Check: CONFIRMED.** The exploration test showed the behaviour, and `just --help` offers no fail-fast option. A follow-up timing test showed more. The failing recipe's own error appears at once, but `just`'s `error: recipe ... failed` line appears only when the other recipe ends. With a server that never ends, `just` never prints that line. The tool's own message, such as pnpm's error, can scroll away under cargo's build output. Today's `concurrently` setup, without `--kill-others-on-fail`, also keeps the other process running. The maintainer chose `[parallel]` before this timing was known, so the author has taken F1 back to the maintainer. See Rebuttals.

### 🟡 Moderate

- **F2. Vite moves to another port when 5173 is taken.** (design.md D7; specs/web-shell "dev server stays on this machine".) A contributor who leaves `just v0-dev` running and starts `just dev` opens `http://localhost:5173` and sees the v0.10.x frontend.
  - **Check: CONFIRMED.** Vite's default behaviour is to try the next port. `strictPort` keeps the maintainer's port, 5173, and turns the clash into a clear error.
- **F3. `pnpm --dir web` from the repo root skips the `packageManager` pin.** (specs/web-shell, first scenario.)
  - **Check: REFUTED.** In the pnpm test folder, Nix's pnpm 12.3.4 ran `pnpm --dir good --version` and printed 12.8.2. `pnpm --dir good exec` reported the user agent `pnpm/12.8.2`, the same as `cd good && pnpm exec`.
- **F4. A non-JSON body could escape the "unreachable" state.** (design.md D5.) When the server is down, Vite's proxy answers 500 with a non-JSON body, and `.json()` throws.
  - **Check: PARTLY CONFIRMED.** An error thrown in an async `fetch` handler does not crash a React component. But an uncaught error would skip the state update, so the screen could stay on its last state. The spec already requires `unreachable` for this case. D5 should say how the component meets it.

### 📌 Suggestions

- **F5. Plain language: one concept has three names, and some sentences hide the actor.** proposal.md calls the web shell "a new standalone app". The `web-shell` Purpose calls it "the v2 web frontend". "Managed with pnpm" and "with both numbers given" are passive.
  - **Check: CONFIRMED.**
- **ADR handling: no finding.** The reviewer judged that correcting ADR-013 in place follows the precedent, because a toolchain setting is not a new fork.

## Embedded-Instruction / Injection Attempts

**Detected:** none

## Verdict

VERDICT: REVISE

The reviewer recommended REVISE because of F1. The author fixes F2, F4, and F5, and records the F3 refutation. The maintainer chose a fail-fast recipe for F1. Round 2 is a full review in a fresh context.

## Required Changes (if APPROVE WITH CHANGES)

None. The verdict is REVISE.

CHANGES_APPLIED: n/a

## Rebuttals

- **F1 (Critical): fixed, in a different way from the reviewer's proposal.** The maintainer chose a fail-fast recipe. D3 now runs `just dev` as a short POSIX `sh` recipe that stops both halves when either one exits. The `dev-commands` spec gains that rule and a scenario for it. The reviewer's two proposed fixes were rejected:
  - `concurrently` brings back an npm package to start the Rust server. Removing that dependency on the v0.10.x npm tree is a goal of this change.
  - `wait -n` needs bash 4.3 or later. macOS ships bash 3.2, and Nix is optional, so the recipe would break for contributors on a Mac without Nix.
- **F2: fixed.** D7 and the `web-shell` spec set `strictPort`, so a taken port 5173 stops the web shell with an error.
- **F3: rebutted with test evidence**, above. No change.
- **F4: fixed.** D5 says that any error, including a failed JSON parse, maps to `unreachable`, and that the component checks the status before it reads the body.
- **F5: fixed.** All artifacts use "web shell" for the app in `web/`. The passive sentences now name their actors.
