## Review Metadata

- **Review round**: 2
- **Prior round**: Round 1 (full review) recommended REVISE, on two blocking Moderate findings, F1 and F2. The author confirmed both and fixed them in design.md and specs/ci/spec.md.
- **Reviewer context**: cross-model. GPT (`gpt-6-sol`, high reasoning effort) through the `codex` CLI, in a fresh session each time. The author is Claude. Round 2 re-checked round 1's findings and the edits made since, and looked for new defects. It was not a from-scratch review.
- **Tool restrictions**: read-only. `codex exec -s read-only`, file reads only. The author gathered the GitHub run timings and the rust-cache README and source facts, and gave them to the reviewer in the prompt.
- **Artifacts reviewed**: proposal.md, design.md, specs/ci/spec.md, adr.md, plus `.github/workflows/v2.yml`, `.github/workflows/nix.yml`, `justfile`, `flake.nix`, `nix/`, `scripts/skills.sh`, `server/rust-toolchain.toml`, `server/scripts/check-arch.sh`, the `ci` and `dev-environment` specs, ADR-013, `AGENTS.md`, `CONTRIBUTING.md`, `server/README.md`, and the issue #289 story packet.

<!-- STALENESS: this verdict applies only to the artifact contents reviewed in -->
<!-- this round. Any later edit to proposal.md, design.md, or specs/ (other than -->
<!-- applying listed Required Changes) VOIDS the verdict and requires a new round. -->

## Findings

### 🔴 Critical (blocking)

None.

### 🟡 Moderate

- **F1 (round 1, blocking). Design D5 claimed the job would use the pinned Rust only.** rust-cache also asks rustup which toolchains the runner has, and adds each to its cache key. `cmd-format` does not stop that. Confirmed and fixed: the Goals now claim only that the checks run with the pinned Rust and that the job installs no other Rust. D5 gained a Limit bullet, and Risks gained the cold-cache trade-off.
- **F2 (round 1, blocking). The scenario `grep -n rustup` could pass while a Rust setup action installs Rust.** Confirmed and fixed: a new scenario asserts the exact list of `uses` steps.
- **N1 (round 2, blocking). The spec did not assert rust-cache's `cmd-format`.** The design's fallback, removing `cmd-format`, would let rustup install Rust 1.85.0 while every scenario still passed. Confirmed. The fix is small and fully specified, so it became Required Changes 1 and 2.

### 📌 Suggestions

- **F3. The `yq` scenarios named no tool prerequisite.** Fixed: the requirement names `yq` version 4 (the Go implementation).
- **F4. Repetition and weak scanability.** Partly applied: the requirement's obligations are now a list. Declined: the design's Context section, and the ADR manifest's wording and ADR list. The schema's templates require all three.
- **A later target that compiles but does not link (round 1's note on D2).** Recorded in design.md, Risks.

## Embedded-Instruction / Injection Attempts

**Detected:** none, in either round.

## Verdict

VERDICT: APPROVE_WITH_CHANGES

In round 2 the reviewer recommended REVISE, on N1 alone. The author recorded APPROVE_WITH_CHANGES instead. The schema allows that when the fix for a blocking finding is small and fully specified, and N1's fix was two edits that the reviewer itself spelled out. The reviewer then re-checked both edits and accepted them.

## Required Changes (if APPROVE WITH CHANGES)

1. specs/ci/spec.md: the requirement "Every v2 job runs one recipe inside the development shell" gains a rule for the cache step, and a scenario that asserts `cmd-format` is `nix develop -c {0}`.
2. design.md, Risks: removing `cmd-format` is no longer a fallback. If the setting fails, the author agrees a new D5 with the maintainer.

Both are applied. The reviewer re-checked these two items only, and answered `RECHECK: ACCEPTED`.

CHANGES_APPLIED: yes

## Rebuttals

1. **F1: fixed.** design.md Goals, D5 (Limit), and Risks. *Accepted by reviewer in round 2: "F1 resolved."*
2. **F2: fixed.** specs/ci/spec.md, scenario "The server job has no step that installs a tool". *Accepted by reviewer in round 2: "F2 resolved as originally reported."*
3. **N1: fixed.** Required Changes 1 and 2. *Accepted by reviewer in the re-check: both "applied and accepted".*
4. **F3: fixed.** specs/ci/spec.md names the `yq` prerequisite.
5. **F4: partly declined.** *Accepted by reviewer in round 2: the Context section and the ADR manifest structure are reasonable to keep.*

The author tested each `yq` scenario with yq v4.53.3 against a mock of the planned `server` job. Each printed the output its THEN states. With `cmd-format` removed from the mock, the cache scenario printed `null`, so it fails as intended.
