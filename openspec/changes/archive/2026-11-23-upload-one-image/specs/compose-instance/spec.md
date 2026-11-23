## MODIFIED Requirements

### Requirement: The app connects and migrates through the foundation's startup path

The `app` service SHALL connect to the `db` service with the foundation's `DATABASE_URL` setting and `sslmode=disable`.
The app SHALL apply migrations through the foundation's startup path. The Compose file SHALL run no other migration step.
Storage initialization SHALL run during app startup without a separate command.

#### Scenario: The database holds the initial migration

- **WHEN** a first start has reached readiness
- **AND** a tester counts the rows of `_sqlx_migrations` where `success` is true, through `docker compose exec db psql`
- **THEN** the count is 4
