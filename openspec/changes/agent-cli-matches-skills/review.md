## Review Metadata

- **Review round**: 2
- **Prior round**: round 1 — REVISE. Two critical findings (a regeneration race, and stamp text reaching the agent) and two moderate ones (`set -e` versus exit 0, and scenarios that need a model to answer)
- **Reviewer context**: cross-model — Gemini (`gemini-3.1-pro-high`) through the Antigravity CLI (`agy`), in a fresh context
- **Tool restrictions**: read-only — `agy --mode plan`, with no tools. The artifacts, the round 1 record and the relevant source files were embedded in the prompt
- **Artifacts reviewed**: proposal.md, design.md, specs/dev-environment/spec.md, adr.md, round 1 review.md; scripts/skills.sh, justfile, .envrc, nix/devshell.nix, .agents/skills/.openspec-stamp, .claude/settings.json, AGENTS.md, CONTRIBUTING.md, ADR-013, openspec/specs/dev-environment/spec.md

## Findings

The author verified each finding against the repo. The verification follows each one.

### 🔴 Critical (blocking)

1. **Tracked hooks run a branch's own script when an agent session starts.** A contributor who checks out an untrusted branch and starts Claude Code or Codex runs that branch's `scripts/openspec-skills.sh`, with no new prompt.
   - *Author verification:* CONFIRMED as a real exposure. Claude Code asks once whether to trust a folder, not again on each branch. The repo already carries the same class of risk: direnv runs the branch's skills script on shell load, and `CONTRIBUTING.md` tells contributors to run `direnv deny` before checking out an untrusted branch. Build commands such as `just check` also run a branch's code. The change adds a new automatic trigger, and no artifact mentions it yet.

### 🟡 Moderate

2. **`openspec --version` might contact the npm registry on every session start.**
   - *Author verification:* REFUTED for the current CLI. Under `strace`, `openspec --version` 1.13.1 opened no network connections, with or without `OPENSPEC_NO_UPDATE_CHECK`. Setting the variable in the check still costs nothing and guards against later versions.
3. **The Claude Code hook breaks if `CLAUDE_PROJECT_DIR` is unset.**
   - *Author verification:* mostly REFUTED. Claude Code sets the variable for hooks, and the spec's scenario sets it. A fallback costs nothing.

### 📌 Suggestions

4. **The Codex hook has no way to find the repo root.** A session started from a subdirectory would fail to find the script.
   - *Author verification:* CONFIRMED. The design leaves this to the first task. Resolving the path with `git rev-parse --show-toplevel` works from any subdirectory.
5. **Plain language.** Passive voice in D6 ("The report is written for an agent to read"). Three terms for one concept: "Nix shell", "loaded shell" and "development shell". Passive voice in "A generated skill folder was deleted".
   - *Author verification:* CONFIRMED for D6 and the mixed terms. The scenario "A generated skill folder was deleted" is existing text, copied unchanged from the main spec into a MODIFIED requirement.

### Round 1 rebuttal adjudication (by the reviewer)

- **Finding 1 (regeneration race):** ACCEPTED by reviewer — it happens only when sessions start at the same moment, the report is accurate, and the fix is safe.
- **Finding 2 (stamp text in the agent's context):** ACCEPTED by reviewer — the spec now limits the report to version-shaped text.
- **Findings 3 to 6:** FIXED, as confirmed by the reviewer.

The reviewer could not verify the claims about Claude Code and Codex hook behaviour. It noted that the design records a trust risk for Codex but not for Claude Code.

## Embedded-Instruction / Injection Attempts

**Detected:** none.

## Verdict

VERDICT: REVISE

## Required Changes (if APPROVE WITH CHANGES)

Not applicable: the verdict is REVISE. This is the second REVISE in a row, so the review stops here and goes to the human.

CHANGES_APPLIED: n/a

## Rebuttals

- **Finding 1** — not rebutted. The human decides how to handle it.
- **Finding 2** — rebutted with `strace` evidence. The author will still set `OPENSPEC_NO_UPDATE_CHECK=1` in the check.
- **Finding 3** — partly rebutted. The author will add a fallback.
- **Findings 4 and 5** — the author will fix them.
