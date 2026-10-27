## ADDED Requirements

### Requirement: Dependabot PR automation accepts both maintained target branches

The default-branch Dependabot auto-merge workflow SHALL accept `pull_request_target` events for PRs targeting `main` or `v0.x`. It SHALL exclude other target branches. Existing Dependabot identity checks and patch/minor eligibility policy SHALL remain in force.

#### Scenario: Maintenance update opens

- **WHEN** Dependabot opens an eligible minor update targeting `v0.x`
- **THEN** the default-branch auto-merge workflow accepts the event

#### Scenario: Main update opens

- **WHEN** Dependabot opens an eligible patch update targeting `main`
- **THEN** the default-branch auto-merge workflow accepts the event

#### Scenario: Update targets another branch

- **WHEN** Dependabot opens an update targeting a branch other than `main` or `v0.x`
- **THEN** the auto-merge workflow does not accept the event

### Requirement: Dependabot PR freshness uses the PR target

After enabling auto-merge, the workflow SHALL compare the PR head against the PR's base branch. It SHALL request a branch update only when that comparison reports that the head is behind, using the expected PR head SHA.

#### Scenario: Maintenance head is behind its base

- **WHEN** auto-merge is enabled for a PR targeting `v0.x` whose head is behind `v0.x`
- **THEN** freshness is checked against `v0.x` and a branch update is requested with the expected head SHA

#### Scenario: Main head is behind its base

- **WHEN** auto-merge is enabled for a PR targeting `main` whose head is behind `main`
- **THEN** freshness is checked against `main` and a branch update is requested with the expected head SHA

#### Scenario: Head is current

- **WHEN** the PR head is current with its base branch
- **THEN** no branch update is requested
