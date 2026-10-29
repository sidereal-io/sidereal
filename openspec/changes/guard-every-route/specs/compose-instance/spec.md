## MODIFIED Requirements

### Requirement: The app exposes only static files and health probes

Without an administrator session, the app SHALL answer only these requests with a status below 400:

- a `GET` or `HEAD` for a web shell file;
- a `GET` or `HEAD` for `/healthz` or `/readyz`;
- the claim and sign-in routes: `GET /api/claim`, `POST /api/claim` while the instance is unclaimed, and `POST /api/session`.

Every other request under `/api` SHALL answer 401 without a session, or 403 when it is a state-changing request from an origin that is not allowed. A `POST`, `PUT`, `PATCH` or `DELETE` to a web shell file or a health probe SHALL answer 405. No request without a session SHALL change the storage volume, and the only database change open to it is the claim.

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
