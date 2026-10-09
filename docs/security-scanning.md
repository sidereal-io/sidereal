# Security scan categories on main

The repository has two code lines: the Rust/React rebuild on `main` and the TypeScript maintenance app on `v0.x`. Fixed categories identify the code line and scan target. A PR uses the same category as its target branch while retaining its own ref and analyzed commit.

## Categories

| Workflow | Target | Category |
| --- | --- | --- |
| `codeql.yml` | main Rust source | `main/codeql/rust` |
| `codeql.yml` | main JavaScript/TypeScript source | `main/codeql/javascript-typescript` |
| `v0-security-scan.yml` | v0.x dependencies | `v0.x/trivy/dependencies` |

CodeQL scans both languages on every PR and push to `main`, without path filters. Categories do not include feature branch names or PR numbers.

The weekly dependency workflow lives on `main` because GitHub runs schedules from the default branch. It checks out `v0.x` at the workspace root and uploads with `ref: refs/heads/v0.x` and the checkout commit SHA. It runs at 06:00 UTC every Monday and on manual dispatch.

The dependency job uploads SARIF before failing on high or critical findings. Both Trivy scans retain the pinned action and default caching; the second sets `skip-setup-trivy: true` to reuse the installed binary. Scan targets, permissions, and severity gates remain unchanged.

## Verification and rollout

Pre-merge validation uses `deli -- actionlint` and `deli -- just check`. The shared change in `openspec/changes/security-scan-categories/` carries the category contract; archive syncs it into `openspec/specs/ci/spec.md`. Pre-merge checks do not prove live baseline comparison.

After the user merges both PRs, follow the pending runtime checklist on #321:

1. Inventory each old setup's tool, category, analysis key, ref, timestamp, and finding count before deletion.
2. Record successful branch analysis URLs and commit SHAs for both new main CodeQL categories. Verify subsequent PR comparisons separately for Rust and JavaScript/TypeScript after their baselines exist.
3. Dispatch the weekly dependency workflow on `main`. Record its analysis URL, `refs/heads/v0.x` ref, checked-out SHA, and repository-mapped result paths.
4. Follow the maintenance runbook in PR #403 for maintenance CodeQL and image baselines, subsequent PR comparisons, and a controlled new vulnerability. Remove the temporary fixture after recording evidence.
5. Retain the old release setup until a successful replacement analysis on the next legitimate release tag. Do not create a release solely to verify the migration.
6. Retire only inventoried setups whose replacements and applicable PR comparisons passed, using Security → Code scanning → Tool status. Keep #321 open until both branch migrations and cleanup are verified.

## Historical results

Historical `trivy-main` uploads describe the previous TypeScript app; the latest inspected v0.10.1. Old CodeQL default-setup analyses also remain in the scan history.

Retire exact obsolete identities. Do not delete all results by language: current `main` still needs JavaScript/TypeScript coverage. Keep the maintenance dependency scan configured throughout replacement.

`main` has no container or release workflow. PR #403 targets `v0.x` and carries its source, container, and release categories. PR #402 carries main configuration and the shared OpenSpec lifecycle. Both remain draft until verification and archive; the user owns both merges.
