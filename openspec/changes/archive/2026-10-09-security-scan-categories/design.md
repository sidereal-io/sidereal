## Context

See `proposal.md` for motivation and `specs/ci/spec.md` for the contract.

Main has separate CodeQL matrix entries for Rust and JavaScript/TypeScript. Its weekly Trivy workflow checks out `v0.x` and explicitly sets the maintenance ref and SHA.

Maintenance CodeQL uses an implicit category and an autobuild step. Two maintenance PR workflows upload container SARIF. Their categories differ from branch uploads.

The Docker test comment reads `steps.security-scan.outcome`. Removing that step requires changing the comment's status logic, not merely deleting its upload.

Release validation ignores unfixed findings and uses `.trivyignore`. Ordinary image scans have a different policy. Existing main Trivy history includes the previous TypeScript app.

## Goals / Non-Goals

**Goals:** preserve scan identity across events, preserve existing failure policies, and verify replacements before setup retirement.

**Non-Goals:** change scanner coverage, severity selection, application dependencies, container platforms, or release policy. Main continues to build and release no containers.

## Decisions

### Use fixed categories for each code line and target

Use the exact category mapping in the delta spec. Main CodeQL can format `main/codeql/${{ matrix.language }}` because its matrix defines only the two supported languages.

All other categories are literals. Do not derive them from `github.ref_name`, PR numbers, or feature branches.

An event-based category breaks baseline matching. A shared language category obscures which code line the setup describes. Category changes are configuration migrations, not application architecture changes.

### Keep one container SARIF owner

Retain both uploads in `docker-build-push.yml`: its PR job and branch merge job use `v0.x/trivy/image`.

The merge job downloads its run's `digests-linux-amd64` artifact separately into a workspace working directory. Its single filename contains the build digest.

Require exactly one 64-character hexadecimal digest. Scan `${REGISTRY}/${IMAGE_NAME}@sha256:<digest>` and retain the event ref and SHA. Fail before scanning if the artifact is missing or malformed.

This preserves the existing AMD64 scan coverage. A mutable branch tag can change between publication and scanning, so it cannot identify the analyzed image reliably.

Remove the SARIF scan and upload from `docker-build-test.yml`. Retain its smoke tests and text report. Give the report step an explicit ID and use its outcome in the existing comment logic.

A failed or missing report means unavailable scan evidence. Never classify it as zero vulnerabilities. Existing successful report rendering stays intact.

Keeping both SARIF owners wastes work and leaves multiple setup identities. Keeping the build-test upload instead would require adding a matching branch baseline there.

### Preserve distinct release and dependency policies

Use `v0.x/trivy/release-image` for release tags. Keep release severity selection, `ignore-unfixed`, `.trivyignore`, failure exit code, and upload-on-failure condition unchanged.

Use `v0.x/trivy/dependencies` for the weekly scan. Preserve its checkout, explicit `refs/heads/v0.x`, checkout commit, permissions, and upload-before-gate order.

Neither PR nor release uploads override their event ref or SHA. A shared category matches results; changing the ref would misattribute them.

### Remove repeated setup without removing policy checks

Maintenance CodeQL sets `build-mode: none` and drops autobuild. Its source analysis needs no application build.

The weekly dependency job retains both scans because they serve different reporting and failure policies. Its second Trivy invocation sets `skip-setup-trivy: true`.

Pin maintenance Trivy invocations to the reviewed full commit already used by the weekly workflow, after verifying its supported inputs. Keep action caching enabled.

The Docker test retains only one Trivy invocation after removing duplicate SARIF. It therefore needs no skip-setup setting.

## Risks / Trade-offs

- A category rename creates a baseline gap → verify target-branch uploads, then rerun PR analysis before declaring migration successful.
- A container finding may lack a repository path → choose a fixture whose location maps to a changed manifest in the PR.
- Trivy databases change over time → record the fixture CVE and scanned image identity when testing; verify its absence from the baseline.
- Mutable branch image tags can race with another push → scan this run's immutable AMD64 build digest and record it with the commit.
- Report failures can leave stale or empty files → use the report step outcome before reading or summarizing its output.
- Cleanup can remove useful coverage → inventory exact setup identities and retain predecessors until replacements work.
- Artifact review can miss another model family's perspective → disclose the review fallback rather than claiming cross-model independence.

## Migration Plan

1. Keep this change's authoritative artifacts on PR #402. PR #403 references the same change and carries only maintenance-branch implementation.
2. Add one commit per implementation task on its target branch. Update the two existing runbooks to describe implemented behavior as tasks complete.
3. Validate workflow syntax and branch attribution. Run required branch checks through `deli`.
4. Complete pre-merge implementation verification and archive this change on the planning branch. Track runtime rollout verification separately on #321.
5. Keep both drafts until the archive and pre-merge checks pass. Make them ready without merging them; the user owns both merges.
6. After merge, record successful replacement CodeQL analyses on each branch. Verify subsequent PR comparisons for main Rust, main JavaScript/TypeScript, and maintenance JavaScript/TypeScript.
7. Dispatch the weekly workflow on main and verify its maintenance ref and SHA. Record analysis URLs and result-check evidence for each verified target.
8. Record a successful maintenance container baseline. Run a temporary PR using the merged workflow configuration and a known vulnerable production dependency.
9. Confirm the new SARIF finding maps to the changed manifest. Confirm PR comparison uses the maintenance baseline without a missing-baseline skip.
10. Remove the fixture branch and PR from the migration test. Keep the fixture out of both final implementation diffs.
11. Verify release replacement on the next legitimate release tag. Until that analysis succeeds, retain the old release setup; never create a release solely for verification.
12. After each target's replacement checks pass, retire only its recorded obsolete setups. Keep #321 open until both migrations and cleanup are complete.

Baseline creation requires merged workflow configuration. Pre-merge review must not claim that post-merge comparison already passed.

Before cleanup, rollback restores the previous workflow categories and upload ownership through a corrective PR. Retained predecessors preserve their old results.

After cleanup, rollback also requires new branch uploads under restored categories. Historical setup deletion is not reversible; preserve the migration inventory and scan evidence first.
