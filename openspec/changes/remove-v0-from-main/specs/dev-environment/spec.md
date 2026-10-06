## REMOVED Requirements

### Requirement: Every reference to the Node version agrees

**Reason**: Replaced by "Contributors and CI name one Node version". The v0.10.x production image leaves `main`, and the server image has no Node, so the image check no longer applies.

**Migration**: None. `.nvmrc` still names Node 26.

## MODIFIED Requirements

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
- **AND** the `ci` workflow's `web` job runs on that pull request

#### Scenario: TypeScript has a newer major release

- **WHEN** Dependabot's weekly run finds a TypeScript release with a higher major version than the one in `web/pnpm-lock.yaml`
- **THEN** no open pull request from Dependabot changes the major version of `typescript` in `web/package.json`

## ADDED Requirements

### Requirement: Contributors and CI name one Node version

The repo SHALL state Node major version 26 wherever it names a Node version for contributors. `.nvmrc` SHALL contain the major version only, so Nix and Node version managers agree. A CI workflow that sets up Node outside the development shell SHALL read `.nvmrc` rather than hold its own copy of the version, so CI can never drift from it.

#### Scenario: A reviewer checks the documented Node version

- **WHEN** a reviewer reads `.nvmrc`
- **THEN** the file contains `26` and nothing else except a trailing newline

#### Scenario: No outdated Node version is left in the docs

- **WHEN** a reviewer searches `AGENTS.md`, `README.md`, `server/README.md`, `web/README.md` and `CONTRIBUTING.md` for a Node major version other than 26
- **THEN** the search finds no match

#### Scenario: A CI workflow that sets up Node reads .nvmrc

- **WHEN** a reviewer runs `grep -rn node-version .github/workflows/`
- **THEN** every matching line reads `node-version-file: '.nvmrc'`
- **AND** no matching line hardcodes a Node version number

### Requirement: Dependabot keeps the v0.x branch up to date

GitHub reads Dependabot's configuration from the default branch only. So `main`'s `.github/dependabot.yml` SHALL declare the v0.10.x updates and aim them at the `v0.x` branch. It SHALL declare exactly three entries with `target-branch` set to `v0.x`: one each for `npm`, `docker`, and `github-actions`, each with `directory` set to `/`. Every other entry SHALL have no `target-branch`, so it updates `main`.

The scenarios below use `yq` version 4 (the Go implementation). The development shell does not provide it, so a reviewer installs it first.

#### Scenario: The configuration declares the v0.x updates

- **WHEN** a reviewer runs `yq '.updates[] | select(."target-branch" == "v0.x") | ."package-ecosystem"' .github/dependabot.yml`
- **THEN** the output is exactly three lines: `npm`, `docker`, and `github-actions`

#### Scenario: Every v0.x entry checks the repo root

- **WHEN** a reviewer runs `yq '.updates[] | select(."target-branch" == "v0.x") | .directory' .github/dependabot.yml`
- **THEN** every line of the output is `/`

#### Scenario: A v0.10.x package has a newer release

- **WHEN** Dependabot's weekly run finds a newer release for a package in the `v0.x` branch's `package.json`
- **THEN** an open pull request from Dependabot targets the `v0.x` branch
- **AND** no open pull request from Dependabot targets `main` for that package
