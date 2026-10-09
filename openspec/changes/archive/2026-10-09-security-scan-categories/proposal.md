## Why

Trivy cannot compare PR findings with `v0.x` because PR and branch scans use different categories. Explicit categories also distinguish the current rebuild from maintenance and historical scan setups.

## What Changes

- Give each code line and scan target an explicit category. PR and target-branch uploads share that category.
- Keep one PR container SARIF owner, while preserving Docker smoke tests and text reports.
- Remove unnecessary CodeQL autobuild on `v0.x`, and reuse Trivy installation within jobs.
- Pin maintenance Trivy action references and restrict maintenance branch triggers to `v0.x`.
- Preserve release filtering, dependency scan attribution, permissions, and severity gates.
- Scan the branch build's immutable digest so findings remain associated with the correct commit.
- Establish replacement baselines before retiring old setups. Verify PR comparison with a controlled vulnerability fixture.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `ci`: explicit scan identities for both code lines, matching PR baselines, preserved scan policies, and verified setup migration.

## Impact

Issue #321 tracks this change. PR #402 carries its planning artifacts and the workflows on `main`; companion PR #403 carries maintenance workflow edits.

Main targets: `.github/workflows/codeql.yml`, `.github/workflows/v0-security-scan.yml`, and the durable CI spec.

Maintenance targets: `.github/workflows/ci.yml`, `docker-build-push.yml`, `docker-build-test.yml`, and `release.yml`.

The change adds no scanner, scan target, container workflow on `main`, or application dependency. Both branches retain their existing severity policies.
