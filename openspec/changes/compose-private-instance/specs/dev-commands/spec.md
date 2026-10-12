## ADDED Requirements

### Requirement: One recipe builds and verifies the Compose instance

`just instance-verify` SHALL build the app image and start a disposable instance from the root `compose.yml`.
It SHALL then run these checks, and SHALL exit with a non-zero status when any one fails:

- Compose refuses to start without `POSTGRES_PASSWORD`;
- the `db` image in `compose.yml` equals the pin in `server/postgres-image.env`;
- the home page, `/healthz`, and `/readyz` return their documented answers;
- the `app` container runs as user ID 10001, and the `db` container publishes no host port;
- the app user can write to the storage volume;
- the migration records are identical after `docker compose down` and a new start;
- the container logs do not contain a generated password, after a successful start and after a failed database login.

The recipe SHALL use its own Compose project name, its own generated password, and its own host port. It SHALL NOT read or change the `.env` file at the repo root.
The recipe SHALL remove its containers, network, and volumes when it ends, whether it passes or fails. It SHALL remove nothing that belongs to another Compose project.
`just check` SHALL NOT run `just instance-verify`.

#### Scenario: The instance is sound

- **GIVEN** a running Docker daemon
- **WHEN** a contributor runs `just instance-verify` on an unchanged checkout
- **THEN** the command exits with status 0
- **AND** `docker ps -a` and `docker volume ls` list nothing that belongs to the recipe's Compose project

#### Scenario: The database volume is not kept

- **WHEN** a tester removes the `db` service's volume mount from `compose.yml`, then runs `just instance-verify`
- **THEN** the command exits with a non-zero status
- **AND** `docker ps -a` and `docker volume ls` list nothing that belongs to the recipe's Compose project

#### Scenario: The web shell is missing from the image

- **WHEN** a tester removes the step that copies the built web shell into the image, then runs `just instance-verify`
- **THEN** the command exits with a non-zero status

#### Scenario: The PostgreSQL pins differ

- **WHEN** a tester changes the `db` image tag in `compose.yml`, then runs `just instance-verify`
- **THEN** the command exits with a non-zero status before it builds the image

#### Scenario: Sam's own instance is running

- **GIVEN** an instance that Sam started from the repo root with their own `.env` file
- **WHEN** a contributor runs `just instance-verify`
- **THEN** the command exits with status 0
- **AND** Sam's containers keep their container IDs, and Sam's volumes still exist
- **AND** `git status --porcelain` and the contents of `.env` are unchanged

#### Scenario: Docker is not running

- **WHEN** a contributor runs `just instance-verify` with no reachable Docker daemon
- **THEN** the command exits with a non-zero status, and its output names Docker

#### Scenario: The gate does not build the image

- **WHEN** a contributor runs `just --show check`
- **THEN** the output contains the line `check: check-server check-web`
