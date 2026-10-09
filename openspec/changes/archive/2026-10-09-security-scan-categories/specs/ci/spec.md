## ADDED Requirements

### Requirement: Scan categories identify the code line and target

Security uploads SHALL use these fixed categories:

| Code line | Target | Category |
| --- | --- | --- |
| main | CodeQL Rust | `main/codeql/rust` |
| main | CodeQL JavaScript/TypeScript | `main/codeql/javascript-typescript` |
| v0.x | CodeQL JavaScript/TypeScript | `v0.x/codeql/javascript-typescript` |
| v0.x | Trivy container | `v0.x/trivy/image` |
| v0.x | Trivy dependencies | `v0.x/trivy/dependencies` |
| v0.x release tag | Trivy release validation | `v0.x/trivy/release-image` |

PR and target-branch uploads SHALL share the category for their code line and scan target. Categories SHALL NOT include a PR number, feature branch name, or release version.

PR uploads SHALL use the PR ref and analyzed commit. Sharing a category SHALL NOT redirect PR results to the target branch ref.

Branch container scans SHALL analyze the immutable AMD64 image digest produced by the same workflow run. Their uploads SHALL retain that run's branch ref and commit SHA.

#### Scenario: Main CodeQL matches its branch baseline

- **WHEN** a successful PR analysis and its main baseline analyze the same language
- **THEN** both analysis records have `main/codeql/rust` or `main/codeql/javascript-typescript`, matching that language
- **AND** the PR record names its PR ref rather than `refs/heads/main`

#### Scenario: Maintenance source analysis has a separate identity

- **WHEN** CodeQL uploads JavaScript/TypeScript results for a PR to `v0.x` and a push to `v0.x`
- **THEN** both analysis records have category `v0.x/codeql/javascript-typescript`
- **AND** neither record uses the main JavaScript/TypeScript category

#### Scenario: Maintenance container results compare with their branch

- **WHEN** a successful `v0.x/trivy/image` baseline exists on `v0.x`
- **AND** a later PR container scan uploads successfully
- **THEN** both analysis records have category `v0.x/trivy/image`
- **AND** the PR result check does not skip because a matching baseline is missing

#### Scenario: Release uploads do not create a category per version

- **WHEN** release validation uploads results for two different release tags
- **THEN** both analysis records use `v0.x/trivy/release-image`
- **AND** each record keeps its own tag ref and analyzed commit

#### Scenario: Two pushes overlap while publishing the branch image

- **WHEN** a newer push replaces the mutable branch image tag before an earlier run scans
- **THEN** the earlier scan uses its own uploaded AMD64 build digest
- **AND** its analysis record retains the earlier run's commit SHA

### Requirement: Maintenance scans preserve existing checks and policies

Maintenance branch and PR triggers SHALL target `v0.x` only. Exactly one workflow SHALL upload PR container SARIF results. Other container checks SHALL preserve smoke tests and text vulnerability reports.

Reports SHALL distinguish scanner failure from a successful scan with no findings. Removing a duplicate SARIF scan SHALL NOT leave references to its missing step outcome.

Release validation SHALL preserve its blocking exit code, ignore-unfixed setting, severity selection, and ignore-file policy. It SHALL attempt SARIF upload after scan failure whenever the result file exists.

Every maintenance Trivy action reference SHALL use a reviewed full commit SHA. CodeQL SHALL analyze maintenance JavaScript/TypeScript without an application build.

#### Scenario: A PR has one container SARIF owner

- **WHEN** a reviewer inspects all maintenance PR workflows
- **THEN** exactly one workflow contains a container SARIF upload
- **AND** the Docker smoke-test command and text vulnerability-report step remain present

#### Scenario: The text report fails

- **WHEN** the Docker test workflow's vulnerability-report step fails
- **THEN** the PR summary does not report a successful security check or zero vulnerabilities

#### Scenario: A release scan finds a blocking vulnerability

- **WHEN** release validation finds a vulnerability blocked by its existing policy
- **THEN** validation exits unsuccessfully
- **AND** the SARIF upload step runs if its result file exists

#### Scenario: Maintenance CodeQL needs no build

- **WHEN** a reviewer inspects the maintenance CodeQL job
- **THEN** its build mode is `none`
- **AND** it has no CodeQL autobuild step

#### Scenario: Maintenance triggers and Trivy pins are explicit

- **WHEN** a reviewer inspects maintenance workflow branch filters and Trivy references
- **THEN** each push or PR branch filter names only `v0.x`
- **AND** every Trivy action reference ends with a full 40-character hexadecimal commit SHA

### Requirement: Setup retirement follows verified replacement

Maintainers SHALL record the exact tool, category, analysis key, ref, timestamp, and finding count before retiring a setup.

Maintainers SHALL verify replacement uploads before deleting retired setups. Branch targets require successful branch analyses; release validation requires a successful analysis on a legitimate release tag.

For every PR-capable target, maintainers SHALL also verify a subsequent PR comparison. This includes both main CodeQL languages, maintenance CodeQL, and maintenance container analysis.

Maintainers SHALL retain the retired release setup until a legitimate release produces replacement evidence. They SHALL NOT create a release solely to verify this migration.

A controlled PR SHALL demonstrate a new container finding under `v0.x/trivy/image`. The fixture SHALL NOT remain in the final implementation branch.

Maintainers SHALL delete only the recorded retired setups. Current main source coverage and the maintenance dependency scan SHALL remain configured.

#### Scenario: A controlled PR introduces a container finding

- **WHEN** a temporary PR introduces a scanner-detectable vulnerability absent from its established baseline
- **THEN** its uploaded container SARIF contains that new finding
- **AND** its Trivy result check compares against `v0.x` without a missing-baseline skip
- **AND** the final implementation diff contains no vulnerable fixture

#### Scenario: Replacement upload has not succeeded

- **WHEN** a branch target has no successful replacement analysis on its target branch
- **THEN** the migration inventory records its predecessor as retained

#### Scenario: Release replacement has no legitimate tag analysis

- **WHEN** no legitimate release has uploaded a successful `v0.x/trivy/release-image` analysis
- **THEN** the migration inventory records the old release setup as retained
- **AND** the migration creates no release solely to retire that setup

#### Scenario: Historical TypeScript main results are retired

- **WHEN** a maintainer removes obsolete main setups after replacement verification
- **THEN** the deleted setup identities are a subset of the recorded retired identities
- **AND** successful analyses remain for both current main CodeQL categories
- **AND** the maintenance dependency setup remains configured

## MODIFIED Requirements

### Requirement: A weekly scan checks v0.x for known vulnerabilities

Dependabot alerts read the default branch only, so they do not cover `v0.x`. The workflow `v0-security-scan.yml` fills that gap for the `v0.x` branch's dependency manifests, such as its npm lockfile. It does not scan container images, and it does not look for secrets. `v0.x` scans its image on every push and pull request.

The workflow lives on `main` because GitHub runs scheduled workflows from the default branch only. It SHALL meet these rules:

- It runs at 06:00 UTC every Monday, and on manual dispatch.
- It checks out the `v0.x` branch at the root of its workspace. It does not check out `main`.
- It scans for known vulnerabilities only.
- It uploads results with the ref `refs/heads/v0.x` and the commit SHA it checked out. The category is `v0.x/trivy/dependencies`. Each result names a file path as it appears on the `v0.x` branch.
- After the upload, it fails the run when it finds a vulnerability rated high or critical.
- Every action it uses from outside GitHub's `actions/` and `github/` organisations is pinned to a full 40-character commit SHA.

The scenarios below use `yq` version 4 (the Go implementation). The development shell does not provide it, so a reviewer installs it first.

#### Scenario: The scan runs every Monday

- **WHEN** a reviewer runs `yq '.on | keys' .github/workflows/v0-security-scan.yml`
- **THEN** the output lists exactly `schedule` and `workflow_dispatch`
- **AND** `yq '.on.schedule[].cron' .github/workflows/v0-security-scan.yml` prints exactly one line: `0 6 * * 1`

#### Scenario: The scan checks out v0.x at the workspace root

- **WHEN** a reviewer runs `yq '.jobs[].steps[] | select(.uses == "actions/checkout*") | .with | [.ref, .path]' .github/workflows/v0-security-scan.yml`
- **THEN** the output lists `v0.x` for `ref`, and `null` for `path`
- **AND** the job has exactly one checkout step

#### Scenario: Every scan step looks for vulnerabilities in the workspace

- **WHEN** a reviewer runs `yq '.jobs[].steps[] | select(.uses == "aquasecurity/trivy-action*") | .with | [."scan-type", ."scan-ref", .scanners]' .github/workflows/v0-security-scan.yml`
- **THEN** every step in the output lists `fs`, `.`, and `vuln`

#### Scenario: The upload names v0.x and the commit it scanned

- **WHEN** a reviewer runs `yq '.jobs[].steps[] | select(.uses == "github/codeql-action/upload-sarif*") | .with' .github/workflows/v0-security-scan.yml`
- **THEN** the output sets `ref` to `refs/heads/v0.x` and `category` to `v0.x/trivy/dependencies`
- **AND** it sets `sha` to the `commit` output of the checkout step

#### Scenario: A high or critical finding fails the run

- **WHEN** a reviewer runs `yq '.jobs[].steps[] | select(.with."exit-code" == "1") | .with.severity' .github/workflows/v0-security-scan.yml`
- **THEN** the output is exactly one line: `HIGH,CRITICAL`
- **AND** that step comes after the upload step in the job

#### Scenario: Third-party actions are pinned

- **WHEN** a reviewer lists every `uses:` value in `v0-security-scan.yml` that does not start with `actions/` or `github/`
- **THEN** every value ends with `@` followed by 40 hexadecimal characters

#### Scenario: A manual run uploads results for the v0.x commit it scanned

- **WHEN** the maintainer records the output of `git ls-remote origin refs/heads/v0.x`
- **AND** runs `v0-security-scan.yml` by hand on `main`, with no push to `v0.x` before the run finishes
- **AND** runs `gh api 'repos/sidereal-io/sidereal/code-scanning/analyses?ref=refs/heads/v0.x' --jq '[.[] | select(.category == "v0.x/trivy/dependencies")][0] | .commit_sha, .created_at'`
- **THEN** the first printed line equals the recorded commit SHA
- **AND** the second printed line is a time later than the moment the run started

#### Scenario: Alerts point at files on v0.x

- **WHEN** a manual run has uploaded at least one result
- **AND** the maintainer runs `gh api 'repos/sidereal-io/sidereal/code-scanning/alerts?ref=refs/heads/v0.x&tool_name=Trivy' --jq '.[].most_recent_instance.location.path'`
- **THEN** for every printed path, `git cat-file -e origin/v0.x:<path>` exits with status 0
