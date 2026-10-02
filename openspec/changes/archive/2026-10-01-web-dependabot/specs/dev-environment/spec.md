## ADDED Requirements

### Requirement: Dependabot proposes web package updates every week

Dependabot SHALL check the packages in `web/package.json` and `web/pnpm-lock.yaml` every Monday at 04:00. When a package has a newer release, Dependabot SHALL open a pull request that updates it. The `/web` entry SHALL group updates exactly as the root `npm` entry does. Dependabot SHALL NOT propose a TypeScript major update for `web/`.

The scenarios below use `yq` version 4 (the Go implementation). The development shell does not provide it, so a reviewer installs it first.

#### Scenario: The configuration declares the web updates

- **WHEN** a reviewer runs `yq '.updates[] | select(."package-ecosystem" == "npm" and .directory == "/web")' .github/dependabot.yml`
- **THEN** the output is exactly one entry
- **AND** that entry has `schedule.interval` set to `weekly`, `schedule.day` set to `monday`, and `schedule.time` set to `04:00`

#### Scenario: The web entry groups updates like the root entry

- **WHEN** a reviewer runs `yq '.updates[] | select(."package-ecosystem" == "npm" and .directory == "/web") | .groups' .github/dependabot.yml`
- **AND** the reviewer runs the same command with `.directory == "/"`
- **THEN** the two outputs are identical

#### Scenario: The web entry ignores TypeScript major updates

- **WHEN** a reviewer runs `yq '.updates[] | select(."package-ecosystem" == "npm" and .directory == "/web") | .ignore' .github/dependabot.yml`
- **THEN** the output is exactly one rule
- **AND** that rule has `dependency-name` set to `typescript` and `update-types` set to `["version-update:semver-major"]`

#### Scenario: A web package has a newer release

- **WHEN** Dependabot's weekly run finds a newer release for a package in `web/package.json`
- **THEN** an open pull request from Dependabot changes `web/pnpm-lock.yaml`
- **AND** that pull request changes files under `web/` only
- **AND** the `v2` workflow's `web` job runs on that pull request

#### Scenario: TypeScript has a newer major release

- **WHEN** Dependabot's weekly run finds a TypeScript release with a higher major version than the one in `web/pnpm-lock.yaml`
- **THEN** no open pull request from Dependabot changes the major version of `typescript` in `web/package.json`
