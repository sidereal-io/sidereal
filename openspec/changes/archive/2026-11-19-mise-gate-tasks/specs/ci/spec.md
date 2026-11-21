## RENAMED Requirements

- FROM: `### Requirement: Every CI job runs its checks as one recipe`
- TO: `### Requirement: Every CI job runs its checks as one task`

- FROM: `### Requirement: Each CI job runs its own check recipe`
- TO: `### Requirement: Each CI job runs its own check task`

## MODIFIED Requirements

### Requirement: Every CI job runs its tool commands through mise

Every step in `.github/workflows/ci.yml` SHALL run each command that needs a tool from `mise.lock` through mise: as `mise exec -- <command>`, or inside a mise task that the step runs as `mise run <task>`. A step that caches cargo's build output SHALL run its own Rust commands through mise, so it installs no Rust of its own.

#### Scenario: Every tool command runs through mise

- **WHEN** a reviewer runs `yq '.jobs[].steps[] | select(has("run")) | .run' .github/workflows/ci.yml | grep -nE '(^|[ (])(cargo|pnpm|just|lets|openspec) ' | grep -v 'mise exec -- '`
- **THEN** the search finds no match

#### Scenario: The cache step uses mise's Rust

- **WHEN** a reviewer runs `yq '.jobs[].steps[] | select(.uses == "Swatinem/rust-cache*") | .with.cmd-format' .github/workflows/ci.yml`
- **THEN** the output is at least one line
- **AND** every line of the output is `mise exec -- {0}`

### Requirement: Every CI job runs its checks as one task

Every job in `.github/workflows/ci.yml` SHALL run its checks as one mise task, and SHALL run no check that the task does not run. The `lets-spec` and `e2e` jobs are exempt from this requirement only. They still install their tools from `mise.lock` and run them through mise.

#### Scenario: A check in the recipe fails

- **WHEN** a pull request changes a file under `server/crates/` so that `cargo fmt --check` reports a difference
- **THEN** the `server` job fails on that pull request

### Requirement: The lets-spec job checks the Let's Spec files

The `lets-spec` job SHALL run `mise exec -- lets sync --check` instead of a mise task. It checks the files that Let's Spec writes, not the project's code, and `mise run check` does not run it.

#### Scenario: The lets-spec job runs one check step

- **WHEN** a reviewer runs `yq '.jobs.lets-spec.steps[] | select(has("run")) | .run' .github/workflows/ci.yml`
- **THEN** the output is exactly one line: `mise exec -- lets sync --check`

#### Scenario: A Let's Spec file differs from the release

- **WHEN** a pull request changes a file under `.agents/skills/lets-plan/`
- **THEN** the `lets-spec` job fails on that pull request

### Requirement: Each CI job runs its own check task

The `compose` job's task is `check-compose`. The `server` job's task is `check-server`. The `web` job's task is `check-web`. The `openspec` job's task is `check-openspec`. Each job SHALL run its task as `mise run <task>`. `mise run check` SHALL run `check-compose`, then `check-openspec`, then `check-server`, then `check-web`, so a contributor and CI run the same gates.

#### Scenario: The compose job has one check step

- **WHEN** a reviewer runs `yq '.jobs.compose.steps[] | select(has("run")) | .run' .github/workflows/ci.yml`
- **THEN** the output is exactly one line: `mise run check-compose`

#### Scenario: The server job has one check step

- **WHEN** a reviewer runs `yq '.jobs.server.steps[] | select(has("run")) | .run' .github/workflows/ci.yml | grep 'check-'`
- **THEN** the output is exactly one line: `mise run check-server`

#### Scenario: The web job has one check step

- **WHEN** a reviewer runs `yq '.jobs.web.steps[] | select(has("run")) | .run' .github/workflows/ci.yml`
- **THEN** the output is exactly one line: `mise run check-web`

#### Scenario: The openspec job has one check step

- **WHEN** a reviewer runs `yq '.jobs.openspec.steps[] | select(has("run")) | .run' .github/workflows/ci.yml`
- **THEN** the output is exactly one line: `mise run check-openspec`

#### Scenario: The local gate runs the same recipe

- **WHEN** a reviewer runs `mise tasks info check --json | jq -r '.run[].task'` at the repo root
- **THEN** the output is exactly four lines, in this order: `check-compose`, `check-openspec`, `check-server`, `check-web`

### Requirement: The web checks run when server or web code changes

A CI job named `web` SHALL run the web checks. The job lives in the workflow file `.github/workflows/ci.yml`. It SHALL run on every pull request, and on every push to `main`, that triggers that workflow. The same files trigger the `server` job.

The checks are the six checks of `mise run check-web`: the type check, the lint, the format check, the `DESIGN.md` lint, the token drift check, and the unit tests of the web shell in `web/`. The job SHALL fail when any check fails.

#### Scenario: A pull request changes web code

- **WHEN** a pull request changes a file under `web/src/`
- **THEN** the `web` job runs on that pull request

#### Scenario: A pull request changes only server code

- **WHEN** a pull request changes files under `server/` only
- **THEN** the `web` job runs on that pull request

#### Scenario: A pull request changes only DESIGN.md

- **WHEN** a pull request changes `DESIGN.md` only
- **THEN** the `web` job runs on that pull request

#### Scenario: DESIGN.md changed, but the token file did not

- **WHEN** a pull request changes a token's value in `DESIGN.md` and does not change `web/src/styles/tokens.css`
- **THEN** the `web` job fails on that pull request

#### Scenario: A pull request changes only documentation

- **WHEN** a pull request changes files under `docs/` only
- **THEN** GitHub does not run the `web` job on that pull request

#### Scenario: A merge to main changes web code

- **WHEN** a commit that changes a file under `web/src/` is pushed to `main`
- **THEN** the `web` job runs on that push

#### Scenario: A web check fails

- **WHEN** a pull request changes a file under `web/src/` so that the format check reports a difference
- **THEN** the `web` job fails on that pull request

### Requirement: The OpenSpec checks run on every pull request

A CI job named `openspec` SHALL run the OpenSpec checks. The job lives in the workflow file `.github/workflows/ci.yml`. It SHALL run on every pull request, and on every push to `main`, that triggers that workflow.

The checks are those of `mise run check-openspec`: strict validation of every spec and every active change. The job SHALL fail when validation reports an error or a warning.

#### Scenario: A pull request changes only a spec

- **WHEN** a pull request changes only a file under `openspec/specs/`
- **THEN** the `openspec` job runs on that pull request

#### Scenario: A spec warning fails the job

- **WHEN** a pull request shortens one spec's Purpose section under `openspec/specs/` to the word `Short.`
- **THEN** the `openspec` job fails on that pull request

#### Scenario: An invalid change fails the job

- **WHEN** a pull request adds a requirement with no scenario to a delta spec in an active change under `openspec/changes/`
- **THEN** the `openspec` job fails on that pull request
