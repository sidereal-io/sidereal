## 1. Main scan configuration — PR #402

- [x] 1.1 Set main CodeQL categories from the spec table. Verify both matrix languages keep matching PR/push identities, unchanged refs, and no path filters.
- [ ] 1.2 Set the weekly dependency category and skip repeated Trivy installation. Verify maintenance ref/SHA, caching, schedule, permissions, pins, and upload-before-gate ordering remain intact.

## 2. Maintenance configuration — PR #403

- [ ] 2.1 Set maintenance CodeQL category and build mode; remove autobuild. Restrict branch/PR filters to `v0.x`; verify tag triggers remain unchanged.
- [ ] 2.2 Give PR and branch container uploads `v0.x/trivy/image`. Remove Docker test SARIF scanning; verify one PR upload owner and preserved smoke-test/report steps.
- [ ] 2.3 Test report outcomes before updating comment logic. Verify successful findings, empty success, failed/missing reports, and failed reports with stale files cannot produce false security success.
- [ ] 2.4 Scan the same run's AMD64 digest artifact instead of its branch tag. Verify valid selection and rejection of missing, multiple, or malformed digest filenames.
- [ ] 2.5 Set the release category and pin every maintenance Trivy action. Verify supported inputs, existing severity/ignore policy, failure exit code, and conditional upload after scan failure.

## 3. Integration verification

- [ ] 3.1 Run workflow validation through `deli -- actionlint` on both branches. Verify categories, upload ownership, event attribution, digest provenance, and absence of dangling removed-step references.
- [ ] 3.2 Run report/digest regression checks and `deli -- npm run check` on maintenance. Run the main `deli -- just check` gate; record results without claiming live migration success.
- [ ] 3.3 Update both security runbooks to match implementation. Verify every category and migration step agrees with this delta spec and preserves branch boundaries.
- [ ] 3.4 Deliver the runtime rollout checklist on #321. Verify it requires exact setup inventory, analysis URLs, branch baselines, all PR comparisons, fixture removal, and legitimate release-tag evidence.

## Completion boundary

These checkboxes cover implementation and delivered rollout instructions. Runtime checks require the merged workflows and remain pending on #321 until evidence exists.

After implementation, use verify and archive on the planning branch. Archive syncs the CI delta into the durable spec in PR #402.

Keep both PRs draft through pre-merge verification and archive. The user owns both merges. Keep #321 open through runtime verification and setup retirement.
