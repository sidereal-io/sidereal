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
