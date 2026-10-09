# Security scan categories on v0.x

The maintenance workflows identify `v0.x` and the scan target with fixed categories. PR and branch container results share a category so GitHub can compare new findings with the maintenance baseline.

## Categories and ownership

| Workflow | Target | Category |
| --- | --- | --- |
| `ci.yml` | JavaScript/TypeScript source | `v0.x/codeql/javascript-typescript` |
| `docker-build-push.yml` | PR and branch containers | `v0.x/trivy/image` |
| `release.yml` | Tagged release validation | `v0.x/trivy/release-image` |

The weekly dependency scan lives on `main` and uses `v0.x/trivy/dependencies`. It uploads against the maintenance ref and checked-out SHA. PR #402 carries that workflow and the archived shared OpenSpec change; PR #403 carries maintenance implementation. Both track #321.

Categories never include PR numbers, feature branches, or release versions. PR and release uploads keep their event ref and SHA; sharing a category does not redirect PR results to `v0.x`.

`docker-build-push.yml` owns container SARIF uploads. PRs scan their local AMD64 image. Branch builds download the same run's `digests-linux-amd64` artifact separately, require exactly one regular file named by a 64-character hexadecimal digest, and scan the published image by that immutable digest. Missing, multiple, malformed, or symbolic-link entries fail selection. Both platforms still build and publish.

`docker-build-test.yml` retains smoke tests and its text vulnerability report, with no SARIF scan or upload. Its PR comment checks the report step outcome before reading the file. Failed or missing reports mean unavailable evidence, including when a stale file exists.

CodeQL uses `build-mode: none` with no autobuild. Branch and PR filters target only `v0.x`. Release tag triggers are unchanged. Release validation preserves its blocking exit code, severity selection, ignore-unfixed setting, `.trivyignore`, and conditional upload after scan failure.

Every Trivy invocation is pinned to `ed142fd0673e97e23eac54620cfb913e5ce36c25` (action v0.36.0). Inputs were checked against that commit; default caching remains enabled. Each maintenance job now invokes Trivy once.

## Verification and rollout

Pre-merge checks are `deli -- actionlint`, `deli -- npm run check`, and `deli -- node --test --test-isolation=none tools/scripts/security-workflows.test.mjs`. CI also runs the report/digest regressions. These checks validate configuration and failure handling; they do not establish live baselines.

After the user merges both PRs, follow the pending runtime checklist on #321:

1. Inventory each old setup's tool, category, analysis key, ref, timestamp, and finding count.
2. Record successful maintenance CodeQL and image branch analyses, including analysis URLs, commit SHA, and the immutable image digest. Verify subsequent PR comparisons for both targets after their baselines exist.
3. Use a controlled PR to introduce a known container vulnerability absent from the baseline. Record its CVE, image identity, repository-mapped SARIF location, and result check. Remove the fixture PR and branch; keep the fixture out of the implementation.
4. Confirm the weekly dependency upload and both main CodeQL branch baselines and subsequent PR comparisons using the main runbook.
5. Retain the previous release setup until the next legitimate release tag has a successful `v0.x/trivy/release-image` analysis. Record its analysis URL and tag SHA. Never create a release solely for migration verification.
6. Retire only inventoried setups whose replacements and applicable PR comparisons passed, using Security → Code scanning → Tool status. Keep #321 open until both migrations and cleanup are verified.

The shared OpenSpec workflow is propose → apply → verify → archive. Pre-merge verification and archive are complete. The shared change is archived at `openspec/changes/archive/2026-10-09-security-scan-categories/` in #402, with its durable CI spec synced. Both PRs are ready for review; the user owns the merges.
