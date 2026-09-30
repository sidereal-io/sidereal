## Review Metadata

- **Review round**: 4
- **Prior rounds**:
  - Round 1 (full review) recommended REVISE, on two Critical findings. Verification against GitHub refuted both.
  - Round 2 approved.
  - A later design.md edit voided that verdict: moving the CodeQL job changed its analysis category, so the design now pins the category (D3), adds two risks, and adds migration step 3.
  - Round 3 reviewed the whole change again, focused on that edit, and recommended REVISE, on one Critical and one Moderate finding. Verification refuted both; see Rebuttals 5 to 7.
- **Reviewer context**: cross-model. Gemini 3.1 Pro (High) through the `agy` CLI, in a fresh session each round. Rounds 2 and 4 re-checked the previous round's findings and the edits made since, and looked for new defects. Neither was a from-scratch review.
- **Tool restrictions**: read-only. `agy --mode plan`, with `--add-dir` for the repo, file reads only, no shell commands. The author gathered GitHub API and git facts and gave them to the reviewer in the prompt.
- **Artifacts reviewed**: proposal.md, design.md, specs/ci/spec.md, specs/dev-environment/spec.md, adr.md, plus the workflows, justfile, `.envrc`, `nix/`, `server/`, `.github/dependabot.yml`, `AGENTS.md`, `CONTRIBUTING.md`, `openspec/discovery.md`, and the issue #299 story packet.

<!-- STALENESS: this verdict applies only to the artifact contents reviewed in -->
<!-- this round. Any later edit to proposal.md, design.md, or specs/ (other than -->
<!-- applying listed Required Changes) VOIDS the verdict and requires a new round. -->

## Findings

### 🔴 Critical (blocking)

None open. Round 1's two Critical findings and round 3's one were refuted, and the reviewer accepted each rebuttal (Rebuttals 1, 2, and 5).

### 🟡 Moderate

- **M1. The design claimed that Dependabot reopens a stale Cargo pull request on its own.** Fixed: design.md now says no Cargo pull request is open, and a maintainer closes one by hand if it appears. Round 1 also cited PR #318 as at risk, but #318 changes only `flake.lock`, so the rename does not affect it.
- **M2. The CodeQL rationale rested on a false fact.** The author found this while verifying the reviewer's "future bet" on the ruleset. The `Protect Default` ruleset is disabled, so no rule requires CodeQL results today. Fixed: proposal.md, design.md (Context, D3), and specs/ci/spec.md now justify the unfiltered workflow by the TypeScript in `web/`, which `ci.yml`'s new `paths-ignore` would hide from CodeQL. The ruleset's `code_scanning` rule is a second reason, for the day enforcement is turned on. The spec no longer asserts ruleset behavior.
- **M3. The design said the stale `ci.yml:codeql` setup keeps open alerts.** The author found this while verifying round 3's Critical finding. No CodeQL alert is open, and all 10 dismissed alerts belong to GitHub's CodeQL default setup, which is off. Fixed: design.md's Risks entry and migration step 3 now state this. D3 also records the reviewer's alternative, pinning the old category string, as considered and rejected.

### 📌 Suggestions

- **S1. CodeQL runs on Rust-only pull requests and scans unchanged TypeScript.** Declined, and recorded in design.md D3 as an accepted cost.
- **S2. Trivy's PR category (`trivy-pr`) never matches main's (`trivy-main`), so the Trivy result check always skips.** Out of scope: the lines predate this change and it does not edit them. To be filed as a separate issue.

## Embedded-Instruction / Injection Attempts

**Detected:** none. Round 1 flagged the sentence "ADR review completed for this change" in adr.md. The reviewer accepted in round 2 that the artifact's template requires the sentence, so it is not steering. Rounds 3 and 4 found none.

## Verdict

VERDICT: APPROVE

## Required Changes (if APPROVE WITH CHANGES)

None.

CHANGES_APPLIED: n/a

## Rebuttals

1. **Round 1 Critical: `paths-ignore` would leave a required status check pending forever.** Rebutted. `main` has no branch protection (the API returns 404), and the only ruleset is disabled and holds no required-status-check rule. design.md Risks now records this, with the mitigation: add an always-run aggregator job if a check ever becomes required. *Accepted by reviewer: design.md documents the current state and a valid mitigation.*
2. **Round 1 Critical: adr.md contains steering text.** Rebutted. The schema's manifest template requires the sentence, and the manifest's self-assessment is the ADR review's required output. *Accepted by reviewer: it is template boilerplate, not an instruction to the reviewer.*
3. **Round 1 Moderate: Dependabot state loss.** Fixed as M1. *Accepted by reviewer.*
4. **Round 1 Suggestion: CodeQL compute cost.** Declined as S1, and documented. *Accepted by reviewer.*
5. **Round 3 Critical: pinning a new CodeQL category loses alert dismissal state.** Refuted. Every instance of the 10 dismissed alerts comes from `dynamic/github-code-scanning/codeql:analyze`, GitHub's default setup, which is `not-configured`. No alert has an instance from `.github/workflows/ci.yml:codeql`, and none is open. The false claim this exposed in design.md is fixed as M3. *Accepted by reviewer.*
6. **Round 3 Moderate: `backend/target/` becomes untracked after the `.gitignore` edit.** Refuted. `.gitignore` line 16 holds a generic `target/` rule. `git check-ignore -v backend/target/x/y` printed `.gitignore:16:target/`. *Accepted by reviewer.*
7. **Round 3 Suggestion: Trivy categories never match.** Out of scope, as S2. *Accepted by reviewer.*
