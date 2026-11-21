## MODIFIED Requirements

### Requirement: CI runs the database fixture through its mise tasks

The `server` and `e2e` jobs SHALL start the database fixture with `mise run db-up`, and SHALL stop it with `mise run db-down` in a step that runs even when an earlier step failed. The `server` job SHALL read its test URL from `mise run db-test-url`. The `e2e` job SHALL pass no database URL to `mise run e2e`, because the e2e task makes its own throwaway database.

#### Scenario: The server job uses the fixture tasks

- **WHEN** a reviewer runs `yq '.jobs.server.steps[] | select(has("run")) | .run' .github/workflows/ci.yml | grep -oE 'mise run db-[a-z-]+'`
- **THEN** the output is exactly three lines, in this order: `mise run db-up`, `mise run db-test-url`, `mise run db-down`

#### Scenario: The e2e job uses the fixture tasks

- **WHEN** a reviewer runs `yq '.jobs.e2e.steps[] | select(has("run")) | .run' .github/workflows/ci.yml | grep -oE 'mise run db-[a-z-]+'`
- **THEN** the output is exactly two lines, in this order: `mise run db-up`, `mise run db-down`

#### Scenario: The e2e job sets no database URL

- **WHEN** a reviewer runs `yq '.jobs.e2e.steps[] | select(has("run")) | .run' .github/workflows/ci.yml | grep -c 'DATABASE_URL'`
- **THEN** the output is `0`

#### Scenario: The fixture stops after a failure

- **WHEN** a reviewer runs `yq '.jobs.server.steps[], .jobs.e2e.steps[] | select(.run == "mise run db-down") | .if' .github/workflows/ci.yml`
- **THEN** the output is exactly two lines, each `always()`
