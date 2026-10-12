## ADDED Requirements

### Requirement: One recipe builds and verifies the Compose instance

`just instance-verify` SHALL build the app image and start a disposable instance from the root `compose.yml`.
It SHALL then run these checks, and SHALL exit with a non-zero status when any one fails:

- Compose refuses to start without `POSTGRES_PASSWORD`;
- the `db` image in `compose.yml` equals the pin in `server/postgres-image.env`;
- the home page, `/healthz`, and `/readyz` return their documented answers;
- the `app` container runs as user ID 10001, and the `db` container publishes no host port;
- the app user can write to the storage volume;
- the `db` container keeps its data in the project's named database volume;
- the migration records are identical after `docker compose down` and a new start;
- the container logs do not contain a generated password, after a successful start and after a failed database login.

The recipe SHALL run the volume check before it removes any container. A failed volume check then leaves no unnamed volume behind.
The recipe SHALL use its own Compose project name, its own image tag, its own generated password, and its own host port.
It SHALL NOT read or change the `.env` file at the repo root. `POSTGRES_PASSWORD` and `SIDEREAL_PORT` values in the caller's shell SHALL NOT reach the disposable instance.
The recipe SHALL give each start at most 180 seconds to become healthy, after the image is built. It SHALL measure that time itself, stop a start that takes longer, and fail.
The recipe SHALL stop before it builds anything when Compose is older than release 2.20. Its output SHALL name the release it needs.
The recipe SHALL stop before it starts anything when its Compose project name already owns a container or a volume that this checkout did not create. It SHALL remove nothing in that case.
Only one run SHALL work on a checkout at a time. A second run in the same checkout SHALL exit with a non-zero status and change nothing.
The recipe SHALL remove its containers, network, volumes, and image when it ends, whether it passes or fails. It SHALL remove nothing that belongs to another Compose project.
`just check` SHALL NOT run `just instance-verify`.

#### Scenario: The instance is sound

- **GIVEN** a running Docker daemon
- **WHEN** a contributor runs `just instance-verify` on an unchanged checkout
- **THEN** the command exits with status 0
- **AND** `docker ps -a` and `docker volume ls` list nothing that belongs to the recipe's Compose project

#### Scenario: The database volume is not kept

- **GIVEN** a saved copy of the output of `docker volume ls -q`
- **WHEN** a tester removes the `db` service's volume mount from `compose.yml`, then runs `just instance-verify`
- **THEN** the command exits with a non-zero status
- **AND** `docker ps -a` lists nothing that belongs to the recipe's Compose project
- **AND** `docker volume ls -q` lists no volume that the saved copy lacks

#### Scenario: The web shell is missing from the image

- **WHEN** a tester removes the step that copies the built web shell into the image, then runs `just instance-verify`
- **THEN** the command exits with a non-zero status within 240 seconds of the image build ending
- **AND** `docker ps -a` and `docker volume ls` list nothing that belongs to the recipe's Compose project

#### Scenario: The database never becomes healthy

- **WHEN** a tester changes the `db` service's health check command in `compose.yml` to `false`, then runs `just instance-verify`
- **THEN** the command exits with a non-zero status within 240 seconds of the image build ending
- **AND** `docker ps -a` and `docker volume ls` list nothing that belongs to the recipe's Compose project

#### Scenario: A second run starts in the same checkout

- **GIVEN** a run of `just instance-verify` that has started its disposable instance
- **WHEN** a contributor starts a second `just instance-verify` in the same checkout
- **THEN** the second command exits with a non-zero status within 10 seconds, and its output says a run is in progress
- **AND** the first run exits with status 0

#### Scenario: Another checkout owns the project name

- **GIVEN** a container that carries the recipe's Compose project label, created outside this checkout
- **WHEN** a contributor runs `just instance-verify`
- **THEN** the command exits with a non-zero status, and its output names the project
- **AND** that container still exists

#### Scenario: Compose is too old

- **WHEN** a contributor runs `just instance-verify` with a Compose release older than 2.20
- **THEN** the command exits with a non-zero status, and its output names release 2.20
- **AND** `docker images` lists no image that belongs to the recipe's Compose project

#### Scenario: The caller's shell holds conflicting settings

- **GIVEN** a shell that exports `POSTGRES_PASSWORD=shellsecret` and `SIDEREAL_PORT=5000`
- **WHEN** a contributor runs `just instance-verify` and reads the disposable `app` container's environment with `docker inspect`
- **THEN** its `DATABASE_URL` does not contain `shellsecret`
- **AND** the disposable instance publishes a host port other than 5000

#### Scenario: The PostgreSQL pins differ

- **WHEN** a tester changes the `db` image tag in `compose.yml`, then runs `just instance-verify`
- **THEN** the command exits with a non-zero status before it builds the image

#### Scenario: Sam's own instance is running

- **GIVEN** an instance that Sam started from the repo root with their own `.env` file
- **WHEN** a contributor runs `just instance-verify`
- **THEN** the command exits with status 0
- **AND** Sam's containers keep their container IDs, and Sam's volumes still exist
- **AND** the image ID that `docker compose images -q app` prints for Sam's project is unchanged
- **AND** `git status --porcelain` and the contents of `.env` are unchanged

#### Scenario: Docker is not running

- **WHEN** a contributor runs `just instance-verify` with no reachable Docker daemon
- **THEN** the command exits with a non-zero status, and its output names Docker

#### Scenario: The gate does not build the image

- **WHEN** a contributor runs `just --show check`
- **THEN** the output contains the line `check: check-server check-web`
