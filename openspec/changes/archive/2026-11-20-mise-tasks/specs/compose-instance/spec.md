<!-- Plain language: Name the actor, condition and observable result. Explain necessary domain terms using the agreed glossary. Keep requirement and scenario headings and WHEN/THEN syntax exactly as required. -->

## MODIFIED Requirements

### Requirement: Deployment verification demonstrates configuration and storage retention

`mise run instance-verify` SHALL demonstrate retention for default volumes and the combined bind-mount installation.
It SHALL compare database migration records, pairing identity, marker bytes, sentinel bytes, and retained settings across container recreation.
The task SHALL use only its own disposable directories, Compose projects, and credentials.
It SHALL NOT initialize, modify, or delete operator-supplied data paths.

#### Scenario: Recreate both installations

- **WHEN** the verification task runs against default volumes and both bind overrides
- **THEN** it exits successfully only when all saved records and bytes remain identical after recreation
- **AND** the app reaches readiness with the same storage pairing

#### Scenario: The shell holds operator mount settings

- **GIVEN** distinctive operator paths and `COMPOSE_FILE` in the caller's environment
- **WHEN** the verification task runs
- **THEN** those paths remain unchanged and no test container mounts them

### Requirement: Deployment verification demonstrates refusal on empty storage

`mise run instance-verify` SHALL demonstrate refusal when a paired database finds an empty directory at its selected storage target.
The refusal case SHALL confirm that startup failed because the marker is missing.

#### Scenario: A paired database finds empty storage

- **WHEN** the verification task starts a paired database with an empty directory at its selected storage target
- **THEN** the app does not start
- **AND** the task exits successfully only when the startup output says the storage marker is missing

### Requirement: Deployment verification demonstrates outage recovery

`mise run instance-verify` SHALL stop the `db` service of a ready default-volume instance.
It SHALL confirm that the `app` container becomes `unhealthy` while `/healthz` still answers `{"status":"ok"}`.
It SHALL start `db` again and confirm that the `app` container becomes `healthy` without a restart.
It SHALL confirm that the app's logs report the outage and the recovery.
It SHALL confirm that the successful migration records and the storage pairing are unchanged after recovery.

#### Scenario: Verify an outage and its recovery

- **WHEN** the verification task runs
- **THEN** it exits successfully only when the app turned `unhealthy`, then `healthy`, with an unchanged `RestartCount`
- **AND** the saved migration records and pairing identity match their values from before the outage
