## REMOVED Requirements

### Requirement: The web shell installs and runs on its own

**Reason**: Replaced by "The web shell installs without a root npm project". The root `package.json` and `package-lock.json` leave `main`, so the checks against them no longer apply.

**Migration**: None. The web shell installs and runs as before.

## ADDED Requirements

### Requirement: The web shell installs without a root npm project

The web shell SHALL live in `web/`, with its own `package.json` and `pnpm-lock.yaml`. Git SHALL track both files. The web shell SHALL install and run with no `package.json` or `package-lock.json` at the repo root. The repo root SHALL have neither file.

#### Scenario: The web shell installs from a fresh clone

- **WHEN** a contributor clones the repo and runs `pnpm --dir web install --frozen-lockfile`
- **THEN** the command exits with status 0
- **AND** the repo root has no `node_modules` folder

#### Scenario: The manifest and lockfile are tracked

- **WHEN** a reviewer runs `git ls-files web/package.json web/pnpm-lock.yaml`
- **THEN** the output lists both files

#### Scenario: The repo root has no npm project

- **WHEN** a reviewer runs `git ls-files package.json package-lock.json`
- **THEN** the output is empty
