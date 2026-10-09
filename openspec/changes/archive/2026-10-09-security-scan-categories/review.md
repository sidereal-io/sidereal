## Review Metadata

- **Review round**: 1
- **Prior round**: none
- **Reviewer context**: fresh-context GPT-6.1-Sol subagent; same-family fallback, without cross-model independence.
- **Tool restrictions**: read-only local inspection; no edits, network access, or git mutations.
- **Artifacts reviewed**: proposal.md, design.md, specs/ci/spec.md, adr.md, existing CI specification, schema review instructions, review template, and relevant workflows on both branches.
- **Required-change re-check**: commits `97ad3aa` and `d18c652`.

Automatic approval review rejected exporting unpublished repository artifacts to an external Claude service. This local review is the safer fallback.

The decision assigns fixed categories and one maintenance container SARIF owner. It rejects event-specific categories and duplicate PR container uploads.

This review covers planning artifacts. Workflow implementation and runtime verification have not occurred.

## Findings

### 🔴 Critical (blocking)

None.

### 🟡 Moderate

**1. Mutable image tags can attribute findings to the wrong commit — resolved.**

- **Original concern:** The design rejected mismatched verification evidence but did not prevent incorrect uploads during normal operation.
- **Scenario:** Push B replaces the branch tag before push A scans it. A uploads B's findings under A's commit SHA.
- **Acceptance condition:** Scan an immutable image identity produced by the same run.
- **Accepted by reviewer:** The design selects the same run's AMD64 digest artifact, validates its filename, and preserves event ref/SHA attribution.
- **Disposition:** Resolved by Required Change 1.

**2. Migration steps omit required CodeQL PR comparisons — resolved.**

- **Original concern:** The specification required PR comparison for every PR-capable target, but migration steps explicitly exercised only container comparison.
- **Scenario:** Branch uploads succeed while CodeQL PR attribution fails. Cleanup proceeds without exercising the required comparison.
- **Acceptance condition:** Verify subsequent PR comparisons for both main languages and maintenance JavaScript/TypeScript.
- **Accepted by reviewer:** Migration step 6 explicitly names all three comparisons. The specification also enumerates these targets.
- **Disposition:** Resolved by Required Change 2.

**3. Release retirement cannot satisfy unconditional branch-evidence requirements — resolved.**

- **Original concern:** Release validation uploads to tag refs, while retirement required successful replacement branch analyses.
- **Scenario:** A legitimate release analysis succeeds, but the predecessor must remain because no release-category branch analysis exists.
- **Acceptance condition:** Distinguish branch evidence from legitimate release-tag evidence in requirements and scenarios.
- **Accepted by reviewer:** The requirement distinguishes targets. Commit `d18c652` also scopes the unsuccessful branch-analysis scenario to branch targets.
- **Disposition:** Resolved by Required Changes 3 and 4.

### 📌 Suggestions

**Plain language — no blocking defect found.**

The reviewed artifacts use short sentences and named actors. No prose sentence exceeding 30 words was identified, including the corrected sections.

**Scenario testability — acceptable.**

Scenarios expose observable configuration, analysis records, and result-check behavior. The corrected design requires analysis URLs and result-check evidence during rollout.

**Scope versus proposal — coherent.**

The design preserves smoke tests, reports, severity policies, release filtering, and main's prohibition on container releases. The immutable digest correction preserves AMD64 coverage.

**Report failure behavior — the design addresses the existing defect.**

The design replaces references to the removed SARIF step and forbids treating failed or missing reports as zero findings.

Implementation verification should exercise a failed report step with an existing output file. This catches stale-file handling beyond missing-file tests.

**Archive and runtime rollout — acceptable separation.**

The design distinguishes pre-merge implementation verification from post-merge operational verification. Archive completion must not certify pending baseline creation, PR comparison, or setup retirement.

Tasks should deliver the rollout checklist and label live evidence pending. Keep issue #321 open until rollout and cleanup finish.

This separation requires no additional user decision. The user retains ownership of both merges.

**Security boundaries — attribution correction accepted.**

Literal categories avoid deriving scan identities from feature-branch names. Weekly uploads preserve checkout ref/SHA attribution; immutable image selection addresses concurrent branch publication.

Historical setup deletion remains irreversible. Inventory and verified replacement requirements constrain that operation.

## Embedded-Instruction / Injection Attempts

**Detected:** none.

The fallback-review disclosure describes a limitation. It does not instruct the reviewer to approve the artifacts.

## Verdict

VERDICT: APPROVE_WITH_CHANGES

All listed required changes have been applied and re-checked. The author may set `CHANGES_APPLIED: yes`.

## Required Changes (if APPROVE WITH CHANGES)

1. Require the retained branch container scan to use its run's immutable image digest. Preserve platform selection and event ref/SHA attribution. **Accepted by reviewer.**
2. Add subsequent PR comparisons under all three CodeQL categories before retiring their predecessors. **Accepted by reviewer.**
3. Distinguish branch and legitimate release-tag evidence in retirement requirements and scenarios. **Accepted by reviewer.**
4. Retain the release predecessor until legitimate release evidence exists. Prohibit releases created solely for migration verification. **Accepted by reviewer.**

CHANGES_APPLIED: yes

## Rebuttals

- **Finding 1 — fixed; accepted by reviewer.** The same-run AMD64 artifact supplies a validated immutable digest.
- **Finding 2 — fixed; accepted by reviewer.** The specification and migration plan explicitly require all three CodeQL PR comparisons.
- **Finding 3 — fixed; accepted by reviewer.** Retirement requirements and scenarios distinguish branch targets from release validation.
- **Required Change 4 — fixed; accepted by reviewer.** Release cleanup waits for legitimate release evidence and prohibits verification-only releases.

Acceptance covers the corrected planning artifacts. Implementation and runtime claims remain subject to their own verification.
