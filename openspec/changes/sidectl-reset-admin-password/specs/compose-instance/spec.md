## MODIFIED Requirements

### Requirement: The app image serves the web shell and the health probes on one origin

The app image SHALL hold the server binary, the `siderealctl` binary and the built web shell. It SHALL set `WEB_ROOT` to the web shell's folder.
`siderealctl` SHALL be on the `PATH` of the app user, so `docker compose exec app siderealctl reset-admin-password` runs it with the same database settings as the server.
Compose SHALL name the built image after its Compose project, so two projects on one host never share an image tag.
The browser SHALL reach the home screen and the health probes on the app's own origin. The app SHALL send no `Access-Control-Allow-Origin` header.

#### Scenario: The browser shows the server's health

- **GIVEN** a ready instance
- **WHEN** a user opens `http://localhost:5000/` in a browser
- **THEN** the page shows the text `healthy` within 5 seconds
- **AND** the browser's network log shows no request to any origin other than `http://localhost:5000`

#### Scenario: Two projects build separate images

- **WHEN** a tester runs `docker compose -p alpha config --images` and `docker compose -p beta config --images`, each with `POSTGRES_PASSWORD` set
- **THEN** the two outputs name different images for the `app` service

#### Scenario: Another origin asks for the app's health

- **GIVEN** a ready instance
- **WHEN** a tester runs `curl -si -H 'Origin: http://localhost:3000' http://localhost:5000/healthz`
- **THEN** the response headers contain no `Access-Control-Allow-Origin` header

#### Scenario: The recovery command is in the image

- **GIVEN** a ready instance
- **WHEN** a tester runs `docker compose exec -T app siderealctl --help`
- **THEN** the exit status is 0 and the output names `reset-admin-password`
