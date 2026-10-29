## MODIFIED Requirements

### Requirement: The app exposes only static files and health probes

Without an administrator session, the app SHALL answer only these requests with a status below 400:

- a `GET` or `HEAD` for a web shell file;
- a `GET` or `HEAD` for `/healthz` or `/readyz`;
- the claim and sign-in routes: `GET /api/claim`, `POST /api/claim` while the instance is unclaimed, and `POST /api/session`.

Every route under `/api` other than the claim and sign-in routes SHALL answer 401 without a session, or 403 when it is a state-changing request from an origin that is not allowed. A `POST`, `PUT`, `PATCH` or `DELETE` to a web shell file or a health probe SHALL answer 405. No request without a session SHALL change the storage volume, and the only database change open to it is the claim.

#### Scenario: A caller sends a mutating request

- **GIVEN** a ready instance
- **WHEN** a caller sends `POST`, `PUT`, `PATCH`, and `DELETE` to `/`, `/healthz`, and `/readyz`
- **THEN** every status is 405
- **AND** the rows of `_sqlx_migrations` and `sidereal_metadata` are unchanged

#### Scenario: A caller reaches the API without a session

- **GIVEN** a ready instance
- **WHEN** a caller sends `GET`, `POST`, `PUT`, `PATCH`, and `DELETE` to `/api/assets` with no cookie
- **THEN** every status is 401
- **AND** the rows of `_sqlx_migrations` and `sidereal_metadata` are unchanged

### Requirement: The server serves the built web shell from a configured folder

The server SHALL read the optional `WEB_ROOT` setting, which names a folder of built web shell files.
With `WEB_ROOT` set, the server SHALL answer `GET` and `HEAD` requests with the matching file under that folder. It SHALL answer `/` with `index.html`.
The server SHALL answer 404 for any other path that names a folder, with or without a trailing slash. It SHALL NOT redirect such a path, and SHALL NOT answer it with that folder's `index.html`.
The server SHALL answer 404 for a path that matches no file. It SHALL NOT answer an unknown path with `index.html`.
A path under `/api` is not a web shell path. The API answers it, as the requirement "The app exposes only static files and health probes" says, and never with a file from `WEB_ROOT`.
The server SHALL NOT let a request path leave `WEB_ROOT`, for example with `..` segments. It SHALL NOT list a folder's contents.
The server SHALL treat the contents of `WEB_ROOT` as trusted. Whoever can write to that folder decides what the server serves.
`/healthz` and `/readyz` SHALL keep their documented answers, whatever files `WEB_ROOT` holds.
With `WEB_ROOT` set, the server SHALL check the folder before it connects to the database. It SHALL exit unsuccessfully when the folder is missing or holds no `index.html`.
The app image SHALL hold the web shell in a folder that the app user cannot write.
Without `WEB_ROOT`, the server SHALL serve no files and SHALL answer as it did before this change.

#### Scenario: The home page loads

- **GIVEN** a server running with `WEB_ROOT` set to a built web shell
- **WHEN** a tester runs `curl -si http://localhost:5000/`
- **THEN** the status is 200
- **AND** the `Content-Type` header starts with `text/html`
- **AND** the body equals the `index.html` file in `WEB_ROOT`

#### Scenario: An unknown path is not found

- **GIVEN** a server running with `WEB_ROOT` set to a built web shell
- **WHEN** a tester requests `GET /no-such-page`
- **THEN** the status is 404
- **AND** the body does not equal `index.html`

#### Scenario: An API path does not fall back to the web shell

- **GIVEN** a server running with `WEB_ROOT` set to a built web shell
- **WHEN** a tester requests `GET /api/assets` with no cookie
- **THEN** the status is 401
- **AND** the body does not equal `index.html`

#### Scenario: A path tries to leave the folder

- **GIVEN** a file named `secret.txt` in the parent folder of `WEB_ROOT`
- **WHEN** a tester runs `curl -si --path-as-is http://localhost:5000/../secret.txt` and `curl -si http://localhost:5000/%2e%2e/secret.txt`
- **THEN** neither status is 200
- **AND** neither body contains the contents of `secret.txt`

#### Scenario: A request names a folder

- **GIVEN** a server running with `WEB_ROOT` set to a built web shell that holds an `assets` folder
- **AND** a tester has added the file `assets/index.html` to it
- **WHEN** a tester runs `curl -si http://localhost:5000/assets` and `curl -si http://localhost:5000/assets/`
- **THEN** both statuses are 404
- **AND** neither response has a `Location` header
- **AND** neither body names a file in that folder

#### Scenario: The app user cannot change the web shell

- **GIVEN** a running instance
- **WHEN** a tester runs `docker compose exec app sh -c 'touch "$WEB_ROOT/probe"'`
- **THEN** the command exits with a non-zero status

#### Scenario: A file shadows a health probe

- **GIVEN** `WEB_ROOT` holds files named `healthz` and `readyz`
- **WHEN** a tester requests `GET /healthz` and `GET /readyz` on a ready server
- **THEN** the bodies are `{"status":"ok"}` and `{"status":"ready"}`

#### Scenario: The folder is missing

- **GIVEN** an empty dedicated database
- **WHEN** Sam starts the server with `WEB_ROOT` set to a path that does not exist
- **THEN** the process exits with a non-zero status, and its output names `WEB_ROOT`
- **AND** the database still holds no application table

#### Scenario: The setting is absent

- **WHEN** Sam starts the server without `WEB_ROOT` and a tester requests `GET /`
- **THEN** the status is 404
- **AND** `GET /healthz` still returns `200 {"status":"ok"}`
