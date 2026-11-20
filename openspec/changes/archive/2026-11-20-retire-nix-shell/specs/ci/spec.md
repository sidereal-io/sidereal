<!-- Plain language: Name the actor, condition and observable result. Explain necessary domain terms using the agreed glossary. Keep requirement and scenario headings and WHEN/THEN syntax exactly as required. -->

## ADDED Requirements

### Requirement: The server checks run when server or web code changes

A CI job named `server` SHALL run the server checks. The job lives in the workflow file `.github/workflows/ci.yml`. It SHALL run on every pull request, and on every push to `main`, that changes one of these files:

- any file under `server/`;
- any file under `web/`;
- `DESIGN.md` at the repo root;
- `.github/workflows/ci.yml`.

#### Scenario: A pull request changes Rust code

- **WHEN** a pull request changes a file under `server/crates/`
- **THEN** the `server` job runs on that pull request
- **AND** the job fails if `cargo clippy --all-targets -- -D warnings` reports a warning

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

## REMOVED Requirements

### Requirement: The server checks run when server code changes

**Reason**: It named `flake.nix`, `flake.lock` and `nix/` as triggers, and those files are deleted.
**Migration**: "The server checks run when server or web code changes" keeps every other trigger and scenario.
