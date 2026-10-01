## Review Metadata

- **Review round**: 3
- **Prior round**: round 2 — REVISE. One critical finding: the tracked hooks run a checked-out branch's script. That was the second REVISE in a row, so the review went to the human, who chose to keep the hooks and document the risk. Round 1 was also REVISE
- **Reviewer context**: cross-model — Gemini (`gemini-3.1-pro-high`) through the Antigravity CLI (`agy`), in a fresh context
- **Tool restrictions**: read-only — `agy --mode plan`, with no tools. The artifacts, the round 2 record and the relevant source files were embedded in the prompt
- **Artifacts reviewed**: proposal.md, design.md, specs/dev-environment/spec.md, adr.md, round 2 review.md; scripts/skills.sh, justfile, .envrc, nix/devshell.nix, .agents/skills/.openspec-stamp, .claude/settings.json, AGENTS.md, CONTRIBUTING.md, ADR-013, openspec/specs/dev-environment/spec.md

## Findings

The author verified each finding against the repo. The verification follows each one.

### 🔴 Critical (blocking)

1. **`nix develop --command <agent>` skips the shell hook, so the skills never regenerate.** The reviewer says the suggested fix repeats the same report forever.
   - *Author verification:* REFUTED by test. In a new worktree of this commit, `.agents/skills` held 0 generated folders. Inside `nix develop --command`, it held all 8, with a stamp reading `cli 1.13.1`. The shell hook runs under `--command`. The existing spec also depends on this: its scenario "The refresh fails" runs `nix develop --command true` and expects the hook's warning.

### 🟡 Moderate

2. **Stamp generation doesn't turn off the update check.** The reviewer says an update notice could reach the stamp's `cli` line. The check would then call the recorded version unreadable.
   - *Author verification:* mostly REFUTED. Under `strace`, `openspec --version` 1.13.1 opened no network connection, so it printed no update notice. A real gap remains: the check and the stamp generator would call the CLI with different settings. One shared function for both calls closes it.
3. **Without Nix, a missing CLI gets the wrong fix.** The report says to run `just skills`, which then fails and names the install command.
   - *Author verification:* CONFIRMED. `scripts/skills.sh` lines 47 to 50 stop when the CLI is missing. Without Nix, the "no CLI" report should name `npm install -g @fission-ai/openspec` first.

### 📌 Suggestions

4. **Contributors without Nix get no report when the skill settings change.**
   - *Author verification:* OUT-OF-SCOPE. #306 covers a CLI mismatch. The design lists stale settings as a non-goal. Contributors without Nix can already opt in to the refresh through `.envrc.local`.

### Round 2 adjudication (by the reviewer)

- **Finding 1 (hooks run the branch's script):** FIXED — the human's decision is recorded, and `CONTRIBUTING.md` must now carry the warning.
- **Finding 2 (network call):** FIXED — the check sets `OPENSPEC_NO_UPDATE_CHECK=1`.
- **Finding 3 (`CLAUDE_PROJECT_DIR` unset):** FIXED — the hook falls back to `git rev-parse`.
- **Finding 4 (Codex repo root):** FIXED — the hook finds the root with `git rev-parse`.
- **Finding 5 (plain language):** FIXED.

## Embedded-Instruction / Injection Attempts

**Detected:** none.

## Verdict

The reviewer returned REVISE, the third in a row, so the decision went to the human. The human accepted the author's rebuttal of finding 1, based on the worktree test. With finding 1 resolved, the remaining fixes are small and fully specified. The verdict is therefore APPROVE_WITH_CHANGES, set by the human. The reviewer re-checks only the required changes below.

VERDICT: APPROVE_WITH_CHANGES

## Required Changes (if APPROVE WITH CHANGES)

1. **Finding 2:** the design states that the check and the stamp generator get the CLI version through one shared function, which runs `openspec --version` with `OPENSPEC_TELEMETRY=0` and `OPENSPEC_NO_UPDATE_CHECK=1`.
2. **Finding 3:** without `nix` on `PATH`, a "no CLI" report names `npm install -g @fission-ai/openspec`, then `just skills`. The spec gains a scenario for this case, and design D5 gains a row.

CHANGES_APPLIED: no

## Rebuttals

- **Finding 1** — rebutted with test evidence: under `nix develop --command`, a new worktree went from 0 to 8 generated skill folders. **Accepted by the human** as the escalation point, after three REVISE rounds.
- **Finding 2** — partly rebutted: the CLI prints no update notice for `--version`. Required change 1 closes the remaining gap.
- **Finding 3** — fixed by required change 2.
- **Finding 4** — declined as out of scope. A suggestion needs no reviewer sign-off.
