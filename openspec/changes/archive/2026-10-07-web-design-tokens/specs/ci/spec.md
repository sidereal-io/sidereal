## MODIFIED Requirements

### Requirement: The server checks run when server code changes

A CI job named `server` SHALL run the server checks. The job lives in the workflow file `.github/workflows/ci.yml`. It SHALL run on every pull request, and on every push to `main`, that changes one of these files:

- any file under `server/`;
- any file under `web/`;
- `DESIGN.md` at the repo root;
- the root `justfile`;
- `flake.nix` or `flake.lock`;
- any file under `nix/`;
- `.github/workflows/ci.yml`.

The checks are the Rust format check, clippy with warnings denied, the tests, and the dependency-direction lint. The job SHALL fail when any check fails.

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

### Requirement: The web checks run when server or web code changes

A CI job named `web` SHALL run the web checks. The job lives in the workflow file `.github/workflows/ci.yml`. It SHALL run on every pull request, and on every push to `main`, that triggers that workflow. The same files trigger the `server` job.

The checks are the six checks of `just check-web`: the type check, the lint, the format check, the `DESIGN.md` lint, the token drift check, and the unit tests of the web shell in `web/`. The job SHALL fail when any check fails.

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
