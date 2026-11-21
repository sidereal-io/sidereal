<!-- Plain language: Name the actor, condition and observable result. Explain necessary domain terms using the agreed glossary. Keep requirement and scenario headings and WHEN/THEN syntax exactly as required. -->

## ADDED Requirements

### Requirement: CI runs the database fixture through its mise tasks

The `server` and `e2e` jobs SHALL start the database fixture with `mise run db-up`, and SHALL stop it with `mise run db-down` in a step that runs even when an earlier step failed. The `server` job SHALL read its test URL from `mise run db-test-url`, and the `e2e` job SHALL read its runtime URL from `mise run db-url`.

#### Scenario: The server job uses the fixture tasks

- **WHEN** a reviewer runs `yq '.jobs.server.steps[] | select(has("run")) | .run' .github/workflows/ci.yml | grep -oE 'mise run db-[a-z-]+'`
- **THEN** the output is exactly three lines, in this order: `mise run db-up`, `mise run db-test-url`, `mise run db-down`

#### Scenario: The e2e job uses the fixture tasks

- **WHEN** a reviewer runs `yq '.jobs.e2e.steps[] | select(has("run")) | .run' .github/workflows/ci.yml | grep -oE 'mise run db-[a-z-]+'`
- **THEN** the output is exactly three lines, in this order: `mise run db-up`, `mise run db-url`, `mise run db-down`

#### Scenario: The fixture stops after a failure

- **WHEN** a reviewer runs `yq '.jobs.server.steps[], .jobs.e2e.steps[] | select(.run == "mise run db-down") | .if' .github/workflows/ci.yml`
- **THEN** the output is exactly two lines, each `always()`

## MODIFIED Requirements

### Requirement: Every CI job runs its tool commands through mise

Every step in `.github/workflows/ci.yml` SHALL run each command that needs a tool from `mise.lock` through mise: as `mise exec -- <command>`, or inside a mise task that the step runs as `mise run <task>`. A step that caches cargo's build output SHALL run its own Rust commands through mise, so it installs no Rust of its own.

#### Scenario: Every tool command runs through mise

- **WHEN** a reviewer runs `yq '.jobs[].steps[] | select(has("run")) | .run' .github/workflows/ci.yml | grep -nE '(^|[ (])(cargo|pnpm|lets|openspec) ' | grep -v 'mise exec -- '`
- **THEN** the search finds no match

#### Scenario: The cache step uses mise's Rust

- **WHEN** a reviewer runs `yq '.jobs[].steps[] | select(.uses == "Swatinem/rust-cache*") | .with.cmd-format' .github/workflows/ci.yml`
- **THEN** the output is at least one line
- **AND** every line of the output is `mise exec -- {0}`

### Requirement: The e2e job builds the server before the end-to-end gate

The `e2e` job SHALL run these steps in order: start the database fixture, build the server, install Chromium with its system libraries, run `mise run e2e`, and stop the fixture. Building first keeps compiling out of Playwright's 300-second startup window. Chromium's system libraries need root, which the `e2e` task does not have, so they install in their own step.

#### Scenario: The server is built before the browser and the gate

- **WHEN** a reviewer runs `yq '.jobs.e2e.steps[] | select(has("run")) | .run' .github/workflows/ci.yml`
- **THEN** a line containing `mise exec -- cargo build -p sidereal-server` comes before a line containing `mise exec -- pnpm exec playwright install --with-deps chromium`
- **AND** that line comes before the line `mise run e2e`

### Requirement: The server checks run when server code changes

A CI job named `server` SHALL run the server checks. The job lives in the workflow file `.github/workflows/ci.yml`. It SHALL run on every pull request, and on every push to `main`, that changes one of these files:

- any file under `server/`;
- any file under `web/`;
- `DESIGN.md` at the repo root;
- `flake.nix` or `flake.lock`;
- any file under `nix/`;
- `.github/workflows/ci.yml`.

#### Scenario: A pull request changes Rust code

- **WHEN** a pull request changes a file under `server/crates/`
- **THEN** the `server` job runs on that pull request
- **AND** the job fails if `cargo clippy --all-targets -- -D warnings` reports a warning

#### Scenario: A pull request changes only the shell's pins

- **WHEN** a pull request changes `flake.lock` only
- **THEN** the `server` job runs on that pull request

#### Scenario: A pull request changes only web code

- **WHEN** a pull request changes files under `web/` only
- **THEN** the `server` job runs on that pull request

#### Scenario: A pull request changes only documentation

- **WHEN** a pull request changes files under `docs/` only
- **THEN** GitHub does not run the `server` job on that pull request

#### Scenario: A merge to main changes Rust code

- **WHEN** a commit that changes a file under `server/crates/` is pushed to `main`
- **THEN** the `server` job runs on that push

#### Scenario: The old workflow names are gone

- **WHEN** a reviewer lists `.github/workflows/`
- **THEN** `ci.yml` exists
- **AND** neither `v2.yml` nor `backend-rs.yml` exists
