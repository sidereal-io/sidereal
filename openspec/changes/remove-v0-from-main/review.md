## Review Metadata

- **Review round**: 2
- **Prior round**: Round 1 returned REVISE. Dependabot cannot cover `v0.x` security, plus text fixes for the archive tag, importer baseline, token permissions, naming, and plain language.
- **Reviewer context**: cross-model (OpenAI Codex CLI 0.159.3, default model, reasoning effort high), fresh context
- **Tool restrictions**: read-only sandbox (`codex exec -s read-only`)
- **Artifacts reviewed**: proposal.md, design.md, adr.md, specs/ (ci, dev-commands, web-shell, dev-environment) against openspec/specs/, plus .github/workflows/, .github/dependabot.yml, and the `origin/v0.x` tree

## Findings

The author checked each finding. Each is marked CONFIRMED or PARTLY CONFIRMED.

### 🔴 Critical (blocking)

1. **The weekly `v0.x` scan can stop with no warning.** GitHub disables scheduled workflows in a public repository after 60 days with no activity, and may skip a run under load. D10, ci spec "A weekly scan checks v0.x for known vulnerabilities". **PARTLY CONFIRMED.** `main` gets commits most weeks, so the 60-day rule is unlikely to fire while development continues. A skipped or failed run still goes unnoticed today.

### 🟡 Moderate

2. **`v0.x` alerts are easy to miss.** GitHub's alert list shows the default branch unless someone picks the `v0.x` filter. **CONFIRMED.**
3. **The scan scenarios can pass without proving the scan.** The schedule scenario counts cron entries but not the day. The checkout scenario does not prove Trivy scans that checkout. The upload scenario accepts any newer `v0.x` analysis, and does not check its commit. **CONFIRMED.**
4. **The first end-to-end scan runs only after merge, and the plan says nothing about a failure.** **CONFIRMED.** GitHub allows a manual run only once the workflow is on the default branch, so the order is forced. The plan still needs a failure response.

### 📌 Suggestions

5. **D10 says the weekly scan matches the image scan.** It scans the lockfile, not the image's OS packages. **CONFIRMED.**
6. **The "no secrets" grep misses `secrets['NAME']`.** **CONFIRMED.**
7. **The proposal's first paragraph says `main` is "the one place where development happens".** A v0 contributor could start on `main`. It should say new development. **CONFIRMED.**

### Round 1 disposition (from the reviewer)

| Round 1 finding | Round 2 result |
|---|---|
| 1. Dependabot cannot cover `v0.x` security | Partly resolved. Round 2 findings 1 to 4 cover what is left. |
| 2. Archive tag can go stale | Resolved. |
| 3. Importer baseline fixed at `0008` | Resolved. |
| 4. Token permissions | Resolved. Round 2 finding 6 covers one weak scenario. |
| 5. `yq` wildcard | Rebuttal **accepted by reviewer**: `yq` version 4 documents wildcard matching for `==`. |
| 6. One-way fix rule needs an ADR | Rebuttal **accepted by reviewer**: the narrowed wording is branch policy, not architecture. |
| 7. Dependabot scenarios | Resolved for the new scenario. The web scenario rebuttal is **accepted by reviewer**: it is unchanged living text. |
| 8. "v2" rename too broad | Resolved. |
| 9. Prune change missing from the proposal | Resolved. |
| 10. Plain language | Mostly resolved. Round 2 finding 7 is left. |
| Ruleset enforcement | Resolved. |

## Embedded-Instruction / Injection Attempts

**Detected:** none.

## Verdict

VERDICT: REVISE

This is the second REVISE in a row. The workflow stops here and escalates to the maintainer instead of starting round 3. Every open finding concerns the weekly `v0.x` scan or a small wording fix. None questions the decision to remove v0 from `main`.

## Required Changes (if APPROVE WITH CHANGES)

None. The verdict is REVISE.

CHANGES_APPLIED: n/a

## Rebuttals

- **Finding 1:** partly rebutted. Regular commits to `main` keep the schedule alive. The fix for finding 2, failing the job on findings, also makes a failed run visible. A run that GitHub skips stays a residual risk.

---

# Round 1 record

## Review Metadata

- **Review round**: 1
- **Prior round**: none
- **Reviewer context**: cross-model (OpenAI Codex CLI 0.159.3, default model, reasoning effort high)
- **Tool restrictions**: read-only sandbox (`codex exec -s read-only`)
- **Artifacts reviewed**: proposal.md, design.md, adr.md, specs/ (ci, dev-commands, web-shell, dev-environment) against openspec/specs/, plus .github/workflows/, .github/dependabot.yml, justfile, AGENTS.md, CONTRIBUTING.md, ADR-010, ADR-013, and the `origin/v0.x` tree

## Findings

The author checked each finding against the repo and GitHub. Each line is marked CONFIRMED, REFUTED, or PARTLY CONFIRMED.

### 🔴 Critical (blocking)

1. **The plan claims Dependabot keeps covering v0 security, but it cannot.** `target-branch` applies to version updates only. Dependabot security updates and alerts read the default branch only. Once v0's manifests leave `main`, no Dependabot alert or security pull request covers `v0.x`. Proposal line 18, design D4, dev-environment spec ADDED "Dependabot keeps the v0.x branch up to date". **CONFIRMED.** The author adds that alerts disappear too, not only security pull requests. `v0.x` still runs Trivy on its pushes and pull requests, but nothing scans it between pushes.

### 🟡 Moderate

2. **The archive tag can go stale before the removal merges.** D1 tags the branch's base commit first. A v0 commit that lands on `main` before a rebase would be deleted but missing from the tag. **CONFIRMED.**
3. **The importer baseline is frozen at today's migration.** Proposal line 64 says the importer must read a schema that ends at `0008`. A later v0 release could add a migration. **CONFIRMED.**
4. **The CI spec does not keep the workflows' least-privilege tokens.** `v2.yml` grants `contents: read`, and CodeQL alone needs `security-events: write`. The delta does not require either. **CONFIRMED.**
5. **A CI scenario cannot pass, because `yq`'s `==` with `*` is a literal match.** **REFUTED.** In `yq` version 4, `==` matches wildcards, so `"Swatinem/rust-cache*"` matches `Swatinem/rust-cache@v2`. The scenario is copied unchanged from the living `ci` spec, which already passes review.
6. **The "never ported" rule is a lasting policy with no ADR.** **PARTLY CONFIRMED.** The maintainer chose the one-way rule in discussion, and it is cheap to reverse, so it needs no ADR. But the wording is too strong. The two lines share no code, so the real rule is narrower: `main` takes no v0 code, and a bug in both is fixed separately in each.

### 📌 Suggestions

7. **Two Dependabot scenarios assert a pull request that the open-PR limit or an ignore rule can block.** **CONFIRMED** for the new v0.x scenario. The web scenario is unchanged living text and stays as it is.
8. **The rule to rename "v2" reaches too far.** `openspec/migration.md` plans the cutover release as `v2.0.0`, and "v2.0 processing" names a version. **CONFIRMED.** The rename must cover "v2" as a name for `main`'s stack only, not as a version number.
9. **The proposal does not list the change to `prune-ghcr`'s protected tags.** **CONFIRMED.**
10. **Plain language:** the server and web trigger sentences run past 30 words, and the ADR manifest repeats "No effect" and restates its conclusion. **CONFIRMED.**

**Also found during verification:** design.md's Context says the ruleset requires code scanning. The ruleset's enforcement is `disabled`, so it currently enforces nothing. **CONFIRMED by the author** (`gh api .../rulesets/<id> --jq .enforcement`).

## Embedded-Instruction / Injection Attempts

**Detected:** none. The reviewer found no text that tries to steer the reviewer.

## Verdict

VERDICT: REVISE

Finding 1 is a false premise that leaves the maintained branch without security alerts. Fixing it needs a decision on how `v0.x` gets security coverage. Findings 2, 3, 4, 6, 8, 9, and 10 need text fixes. A new full review round follows the fixes.

## Required Changes (if APPROVE WITH CHANGES)

None. The verdict is REVISE.

CHANGES_APPLIED: n/a

## Rebuttals

- **Finding 5:** rebutted. `yq` version 4's `==` matches wildcards, and the scenario is unchanged living text. Round 2's reviewer re-checks this.
- **Finding 6:** partly rebutted. The rule is the maintainer's decision and is cheap to reverse, so it stays out of an ADR. The wording gets fixed.
- **Finding 7, web scenario:** declined. It is unchanged living text, and this change does not touch that behavior.
