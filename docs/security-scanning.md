# Security scan categories on main

Status: draft category migration for #321. The workflow edits are pending.

The repository has two code lines: the Rust/React rebuild on `main` and the TypeScript maintenance app on `v0.x`. Categories identify the code line and scan target. A pull request uses the same category as its target branch.

## Proposed categories

| Workflow | Target | Category |
| --- | --- | --- |
| `codeql.yml` | main Rust source | `main/codeql/rust` |
| `codeql.yml` | main JavaScript/TypeScript source | `main/codeql/javascript-typescript` |
| `v0-security-scan.yml` | v0.x dependencies | `v0.x/trivy/dependencies` |

Use fixed category values. A feature branch name or PR number must not become a category.

CodeQL continues to scan both languages on every PR and push to `main`, without a path filter. Its categories currently use `/language:<language>`.

The weekly dependency workflow runs on `main` because scheduled workflows run from the default branch. It checks out `v0.x`. Its upload must keep `ref: refs/heads/v0.x` and the checked-out commit SHA. Changing its category must not file the results under `main`.

The dependency category currently uses `v0.x-dependencies`. The job continues to upload findings before failing on high or critical vulnerabilities. Its second scan can reuse the installed Trivy binary. Keep caching, action pins, permissions, and schedule intact.

## Baseline migration

1. Record each existing setup's tool, category, analysis key, ref, timestamp, and finding count.
2. Change the workflow categories. Update the dependency-category requirement and scenario in `openspec/specs/ci/spec.md` in the same commit.
3. Check workflow syntax with `deli -- actionlint`. Compare the PR and push categories for each CodeQL language.
4. After merge, confirm successful `main` analyses under both new CodeQL categories. Run the weekly workflow and confirm its results use the new dependency category under `v0.x`.
5. Re-run a PR analysis after the new branch baseline exists. Confirm the result check compares against that baseline.
6. Remove retired setups through Security → Code scanning → Tool status only after their replacements work. Review exact setup identities before deletion.

A category rename alone does not prove that PR comparison works. New branch uploads establish the baselines; later PR uploads verify the comparison.

## Historical results

Historical `trivy-main` uploads describe the previous TypeScript app. The latest such upload inspected v0.10.1, not the current rebuild. Old CodeQL default-setup analyses also remain in the scan history.

Retire only obsolete setup identities. Do not delete all CodeQL results by language: current `main` still needs JavaScript/TypeScript coverage. Keep the maintenance dependency setup while replacing its category.

`main` has no container or release workflow. #321 covers the maintenance branch's source, container, and release categories in companion PR #403 targeting `v0.x`. Both PRs track #321; complete both migrations before closing it.
