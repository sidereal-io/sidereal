## MODIFIED Requirements

### Requirement: The operator supplies the database password

The bundled entry file `compose.yml` SHALL read the database password from `POSTGRES_PASSWORD`. It SHALL hold no default password.
Bundled mode SHALL refuse to start when `POSTGRES_PASSWORD` is missing or empty.
External mode SHALL NOT require `POSTGRES_PASSWORD`.
The repo SHALL hold `.env.example`, which lists every supported Compose setting, identifies which mode needs each setting, and leaves `POSTGRES_PASSWORD` empty.
Git SHALL ignore `.env`.
The supported password alphabet SHALL be ASCII letters and digits. `.env.example` SHALL state that alphabet beside the setting.
A password outside that alphabet SHALL NOT make the app print the password or the connection URL.

#### Scenario: The password is missing

- **GIVEN** a Compose project name that owns no container
- **AND** no `.env` file, and no `POSTGRES_PASSWORD` in the environment
- **WHEN** Sam runs bundled mode with `docker compose up -d` under that project name
- **THEN** the command exits with a non-zero status
- **AND** its output names `POSTGRES_PASSWORD`
- **AND** `docker ps -a -q --filter label=com.docker.compose.project=<name>` exits with status 0 and prints nothing

#### Scenario: The password is empty

- **GIVEN** a Compose project name that owns no container
- **AND** a `.env` file that is an unchanged copy of `.env.example`, and no `POSTGRES_PASSWORD` in the environment
- **WHEN** Sam runs bundled mode with `docker compose up -d` under that project name
- **THEN** the command exits with a non-zero status
- **AND** `docker ps -a -q --filter label=com.docker.compose.project=<name>` exits with status 0 and prints nothing

#### Scenario: The repo holds no password

- **WHEN** a reviewer runs `grep -n '^POSTGRES_PASSWORD=' .env.example`
- **THEN** the output is exactly one line, and nothing follows its `=`
- **AND** `grep -c 'POSTGRES_PASSWORD:-' compose.yml` prints `0`

#### Scenario: The password holds URL delimiters

- **GIVEN** new volumes, and `POSTGRES_PASSWORD` set to `Zq7@evil.invalid/x?sslmode=disable#`
- **WHEN** Sam runs bundled mode with `docker compose up -d`, and a tester waits 60 seconds
- **THEN** `docker inspect` reports a health status other than `healthy` for the `app` container
- **AND** the output of `docker compose logs app` contains neither `Zq7@evil` nor `postgres://`

#### Scenario: The example file states the alphabet

- **WHEN** a reviewer reads the comment above `POSTGRES_PASSWORD` in `.env.example`
- **THEN** it says the password must hold only letters and digits

#### Scenario: Git ignores the settings file

- **WHEN** a reviewer runs `git check-ignore .env` at the repo root
- **THEN** the command exits with status 0

