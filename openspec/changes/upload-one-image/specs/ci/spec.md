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

- **WHEN** a reviewer runs `yq '[(.env, .jobs.e2e.env, .jobs.e2e.steps[].env) | select(. != null) | keys | .[] | select(. == "DATABASE_URL")] | length' .github/workflows/ci.yml`
- **THEN** it exits with status 0 and prints `0`, because no workflow, job or step environment block that the `e2e` job uses sets `DATABASE_URL`
- **AND** `yq '[.jobs.e2e.steps[] | select(has("run")) | .run | select(test("DATABASE_URL"))] | length' .github/workflows/ci.yml` exits with status 0 and prints `0`, because no run command of the `e2e` job names it

#### Scenario: The fixture stops after a failure

- **WHEN** a reviewer runs `yq '(.jobs.server.steps[], .jobs.e2e.steps[]) | select(.run == "mise run db-down") | .if' .github/workflows/ci.yml`
- **THEN** the output is exactly two lines, each `always()`

### Requirement: The e2e job builds the server before the end-to-end gate

The `e2e` job SHALL run these steps in order: start the database fixture, build the server, install Chromium with its system libraries, run `mise run e2e`, and stop the fixture. Building first keeps compiling out of the end-to-end runner's 300-second wait for the server's listening line. Chromium's system libraries need root, which the `e2e` task does not have, so they install in their own step.

#### Scenario: The server is built before the browser and the gate

- **WHEN** a reviewer runs `yq '.jobs.e2e.steps[] | select(has("run")) | .run' .github/workflows/ci.yml`
- **THEN** a line containing `mise exec -- cargo build -p sidereal-server` comes before a line containing `mise exec -- pnpm exec playwright install --with-deps chromium`
- **AND** that line comes before the line `mise run e2e`
