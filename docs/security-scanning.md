# Security scan categories on v0.x

Status: draft category migration for #321. The workflow edits are pending.

The maintenance app uses explicit categories that identify `v0.x` and the scan target. A pull request's container scan uses the same category as the maintenance branch, so GitHub can compare new findings with the branch baseline.

## Proposed categories

| Workflow | Target | Category |
| --- | --- | --- |
| `ci.yml` | JavaScript/TypeScript source | `v0.x/codeql/javascript-typescript` |
| `docker-build-push.yml` | PR and branch containers | `v0.x/trivy/image` |
| `release.yml` | Tagged release validation | `v0.x/trivy/release-image` |

The weekly dependency scan lives on `main`. Its proposed category is `v0.x/trivy/dependencies`, and it uploads against the maintenance ref and checked-out SHA. Companion PR #402 covers that workflow. Both PRs track #321.

Use fixed categories. PR numbers, feature branch names, and release tag names must not create new category values.

## Workflow ownership

`docker-build-push.yml` owns container SARIF uploads for both PRs and branch pushes. Both uploads change from their current `trivy-pr` and `trivy-v0.x` categories to `v0.x/trivy/image`.

Remove the duplicate SARIF scan and upload from `docker-build-test.yml`. Preserve its container smoke tests and text vulnerability report. Update summaries and step-outcome checks that reference the removed scan. A report failure must not be reported as a successful security check.

CodeQL uses `build-mode: none` for JavaScript/TypeScript. Remove the unnecessary autobuild step. Restrict the maintenance workflows' branch and PR triggers to `v0.x`.

Release validation retains its blocking exit code and ignore-unfixed policy. Its category stays separate from ordinary image scans because it uses different filtering and uploads to a tag. Keep its upload-on-failure condition.

Pin Trivy actions to a reviewed full commit, including the report step's current `master` reference. Check inputs against that commit. Retain caching and reuse the installed binary when a job invokes Trivy again.

## Baseline migration

1. Record existing setup identities, refs, timestamps, and finding counts before making changes.
2. Check workflow syntax with `deli -- actionlint`. Confirm PR and branch uploads use exactly the same container category and that there is one PR SARIF owner.
3. Run `deli -- npm run check` before submitting the implementation for final review.
4. After merge, confirm successful `v0.x` uploads under the new source and container categories.
5. Re-run a PR scan after the new branch baseline exists. Confirm its Trivy result check compares against the branch instead of skipping for a missing baseline.
6. Use a controlled PR that introduces a container vulnerability to demonstrate a new finding. Keep this fixture out of the final branch.
7. Remove retired setups through Security → Code scanning → Tool status only after successful replacement uploads and PR comparison. Identify each setup precisely before deletion.

Never upload PR results using the maintenance branch ref. PR results belong to the PR; sharing a category supplies the matching baseline.

The draft PR remains open through implementation and verification. Under the maintenance guide, a human applies `spec/ready` before workflow implementation. No severity policy or scan target changes in this migration.
