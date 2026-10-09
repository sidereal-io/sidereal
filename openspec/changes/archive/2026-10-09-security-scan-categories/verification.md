# Verification: security-scan-categories

Verified on 2026-10-09 against main implementation `2cd9f69115faf1c8256424ff438a96ac0fd42d09` and maintenance implementation `a9dc357507b4517de37be2006a7a39513103e74a`.

Scope: both draft PRs, #402 and #403. All six planning artifacts were read. This report verifies implementation before archive; it does not certify post-merge migration.

## Summary

| Dimension | Result |
| --- | --- |
| Completeness | 11/11 tasks complete; all 4 requirements mapped |
| Correctness | Configuration and regression checks passed; all 22 scenarios mapped below |
| Coherence | Category, ownership, policy, and migration decisions followed |
| Findings | 0 critical issues; 1 rollout warning; 0 suggestions |

## Requirement evidence

| Requirement | Implementation evidence |
| --- | --- |
| Scan categories identify the code line and target | Main `.github/workflows/codeql.yml:41`; maintenance `ci.yml:57`, `docker-build-push.yml:119` and `:303`, `release.yml:113`; main `v0-security-scan.yml:48`. PR uploads retain event attribution. Branch image selection at maintenance `docker-build-push.yml:261` downloads this run's AMD64 artifact and validates an immutable digest. |
| Maintenance scans preserve existing checks and policies | Maintenance `ci.yml:52` uses build mode none without autobuild. Only `docker-build-push.yml` owns PR container SARIF. `docker-build-test.yml:103` preserves smoke tests and its report; `:175` checks the report outcome before reading a file. Release configuration differs from its base only in the action pin and category. |
| Setup retirement follows verified replacement | Both `docs/security-scanning.md` runbooks and the pending checklist on #321 require exact setup inventory, successful replacement evidence, all applicable PR comparisons, fixture removal, and legitimate release evidence before cleanup. No setup was retired and no release was created. Runtime acceptance remains pending. |
| A weekly scan checks v0.x for known vulnerabilities | Main `v0-security-scan.yml:10`, `:27`, `:34`, and `:45` preserve schedule, checkout, vulnerability-only target, and maintenance ref/SHA. Upload precedes the unchanged high/critical gate. Both action references remain pinned; the second scan reuses installation. |

The two branches contain no application changes or vulnerable fixture. Main has no new container or release workflow. Required durable-spec synchronization remains part of archive, as the task completion boundary specifies.

## Scenario coverage

“Rollout pending” means the scenario needs merged branch configuration or a legitimate release. Its implementation and instructions were inspected, but its runtime result is unverified.

| Scenario | Evidence or remaining check |
| --- | --- |
| Main CodeQL matches its branch baseline | Both PR categories and PR ref verified in actual analyses; branch baseline and subsequent comparison are rollout pending. |
| Maintenance source analysis has a separate identity | Actual PR analysis uses the maintenance category and PR ref; new branch evidence is rollout pending. |
| Maintenance container results compare with their branch | Both uploads use the same literal category; actual PR upload succeeded. Baseline comparison is rollout pending. |
| Release uploads do not create a category per version | Fixed category and unchanged tag trigger verified; legitimate release evidence is rollout pending. |
| Two pushes overlap while publishing the branch image | Scan consumes a validated same-run AMD64 digest, never the branch tag; executable digest-selection regressions passed. |
| A PR has one container SARIF owner | All maintenance workflows inspected; one PR upload owner. Smoke and report steps remain. |
| The text report fails | Executed workflow comment script handles failed, skipped, cancelled, missing, empty, and stale report cases correctly. |
| A release scan finds a blocking vulnerability | Existing exit code, severity, ignore-unfixed, ignore file, and conditional upload preserved exactly; live release validation is rollout pending. |
| Maintenance CodeQL needs no build | Build mode none, no autobuild; published PR CodeQL job succeeded. |
| Maintenance triggers and Trivy pins are explicit | All branch/PR filters name only v0.x; every maintenance Trivy action uses the reviewed 40-character SHA. |
| A controlled PR introduces a container finding | Fixture creation, mapped SARIF finding, comparison, and removal are rollout pending on #321. |
| Replacement upload has not succeeded | Checklist retains predecessors until successful branch replacement and comparison evidence exists. |
| Release replacement has no legitimate tag analysis | Checklist retains release predecessor and forbids verification-only releases. |
| Historical TypeScript main results are retired | Exact inventory and constrained retirement are rollout pending; no deletion performed. |
| The scan runs every Monday | Parsed schedule and event keys match the spec. |
| The scan checks out v0.x at the workspace root | Parsed checkout ref is v0.x; no checkout path override. |
| Every scan step looks for vulnerabilities in the workspace | Both parsed scan inputs are fs, `.`, and vuln. |
| The upload names v0.x and the commit it scanned | Parsed category, maintenance ref, and checkout commit expression match the spec. |
| A high or critical finding fails the run | Exit code 1 and HIGH,CRITICAL verified; gate follows upload. |
| Third-party actions are pinned | Both weekly Trivy references are full commit SHAs; caching remains enabled. |
| A manual run uploads results for the v0.x commit it scanned | Dispatch and analysis ref/SHA confirmation are rollout pending. |
| Alerts point at files on v0.x | Workspace checkout supports correct paths; actual result-path verification is rollout pending. |

## Validation

- OpenSpec strict validation passed; apply state is `all_done` with 11/11 tasks.
- Actionlint v1.7.12 passed across all workflows on both branches during verify.
- All 21 executable report/digest regressions passed during verify through `deli`. CI also runs these regressions.
- Parsed workflow assertions passed for categories, ref/SHA attribution, upload ownership, branch filters, same-run artifact selection, platforms, caching, and release policy. Release content matched the base after only the intended pin/category substitutions.
- Main `deli -- just check` passed during apply: formatting, clippy, Rust tests, architecture check, web checks, and 84 web tests. No server or web source changed afterward.
- Maintenance `deli -- npm run check` passed during apply. Its published CI pipeline, CodeQL, container PR build, type checking, and Docker smoke-test/report job succeeded.

Published main evidence: [CodeQL run 37945812496](https://github.com/sidereal-io/sidereal/actions/runs/37945812496). Maintenance evidence: [CI run 37945812864](https://github.com/sidereal-io/sidereal/actions/runs/37945812864), [container build run 37945812796](https://github.com/sidereal-io/sidereal/actions/runs/37945812796), and [Docker test run 37945812803](https://github.com/sidereal-io/sidereal/actions/runs/37945812803).

Code-scanning API records confirm these uploads have empty errors and retain PR merge refs:

| Analysis ID | Category | Ref | Analyzed merge SHA |
| --- | --- | --- | --- |
| 1923971976 | main/codeql/rust | refs/pull/402/merge | 49e2278b2608e19b42bfb026651eeefac9416d51 |
| 1923965129 | main/codeql/javascript-typescript | refs/pull/402/merge | 49e2278b2608e19b42bfb026651eeefac9416d51 |
| 1923966157 | v0.x/codeql/javascript-typescript | refs/pull/403/merge | 1a9d14713a2d63988a3f3044f8c72a946d0912d0 |
| 1923972847 | v0.x/trivy/image | refs/pull/403/merge | 1a9d14713a2d63988a3f3044f8c72a946d0912d0 |

## Findings

**Critical:** none.

**Warning — runtime migration is unverified.** The maintenance [Trivy result check](https://github.com/sidereal-io/sidereal/runs/113872434404) reports “2 configurations not found”: legacy `v0.x-dependencies` and `trivy-v0.x` configurations. New PR upload success therefore does not establish successful comparison. Branch image publication/digest download, weekly dispatch, release replacement, the controlled vulnerability demonstration, and setup cleanup also need runtime evidence.

Recommendation: after both user-owned merges, execute the pending #321 checklist. Record branch baselines, rerun PR comparisons for all four PR-capable targets, verify the controlled finding, dispatch the weekly scan, and wait for legitimate release evidence before retiring exact predecessor setups. Investigate any remaining comparison warning before marking migration complete.

**Suggestions:** none.

## Assessment

No critical issues. Ready for OpenSpec archive with the recorded rollout warning. Archive may sync the durable specification and prepare both PRs for review; it must leave #321 open and must not claim live migration success. Both PRs remain draft until archive. The user owns both merges.
