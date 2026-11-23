<!-- Plain language: Name the actor, condition and observable result. Explain necessary domain terms using the agreed glossary. Keep requirement and scenario headings and WHEN/THEN syntax exactly as required. -->

## ADDED Requirements

### Requirement: The web shell answers an asset page address with index.html

With `WEB_ROOT` set, the server SHALL answer `GET` and `HEAD` requests for
`/assets/<id>` with status 200 and the `index.html` file in `WEB_ROOT`, where
`<id>` is 36 characters: lowercase hexadecimal digits in groups of 8, 4, 4, 4
and 12, separated by hyphens. It SHALL do so whether or not an Asset has that
id, and without a session. The answer SHALL hold no library data.

#### Scenario: The asset page address loads the web shell
- **GIVEN** a server running with `WEB_ROOT` set to a built web shell
- **WHEN** a tester runs `curl -si http://localhost:5000/assets/0b6f3c2e-8d1a-4f7b-9c3e-2a5d6e7f8091` with no cookie
- **THEN** the status is 200
- **AND** the `Content-Type` header starts with `text/html`
- **AND** the body equals the `index.html` file in `WEB_ROOT`

#### Scenario: A HEAD request for the asset page address
- **GIVEN** a server running with `WEB_ROOT` set to a built web shell
- **WHEN** a tester runs `curl -sI http://localhost:5000/assets/0b6f3c2e-8d1a-4f7b-9c3e-2a5d6e7f8091` with no cookie
- **THEN** the status is 200, the `Content-Type` header starts with `text/html`, and the response has no body

### Requirement: An asset page address is reserved in the web root

A file or folder in `WEB_ROOT` whose path is an asset page address SHALL NOT
be served; the address SHALL answer with `index.html`. Every other file in
`WEB_ROOT` SHALL still be served at its own path.

#### Scenario: The web shell's own files still load
- **GIVEN** a server running with `WEB_ROOT` set to a built web shell
- **WHEN** a tester requests each script and stylesheet path that `index.html` names, with `GET` and with `HEAD`
- **THEN** each status is 200 and each `GET` body equals the file at that path in `WEB_ROOT`

#### Scenario: A file at an asset page address is not served
- **GIVEN** a server running with `WEB_ROOT` set to a built web shell
- **AND** a tester has added a file at `assets/0b6f3c2e-8d1a-4f7b-9c3e-2a5d6e7f8091` in it
- **WHEN** a tester requests `/assets/0b6f3c2e-8d1a-4f7b-9c3e-2a5d6e7f8091`
- **THEN** the status is 200 and the body equals the `index.html` file in `WEB_ROOT`, not the added file

#### Scenario: Other paths under /assets are not asset pages
- **GIVEN** a server running with `WEB_ROOT` set to a built web shell
- **WHEN** a tester requests `/assets/not-an-id`, `/assets/0B6F3C2E-8D1A-4F7B-9C3E-2A5D6E7F8091` and `/assets/0b6f3c2e-8d1a-4f7b-9c3e-2a5d6e7f8091/original`
- **THEN** each status is 404
- **AND** no body equals `index.html`

## MODIFIED Requirements

### Requirement: The web shell answers 404 for folders and unknown paths

The server SHALL answer 404 for any path other than `/` or an asset page address that names a folder, with or without a trailing slash. It SHALL NOT redirect such a path, and SHALL NOT answer it with that folder's `index.html`.
The server SHALL answer 404 for a path that matches no file, except an asset page address, which the requirement "The web shell answers an asset page address with index.html" covers. It SHALL NOT answer any other unknown path with `index.html`.

#### Scenario: An unknown path is not found

- **GIVEN** a server running with `WEB_ROOT` set to a built web shell
- **WHEN** a tester requests `GET /no-such-page`
- **THEN** the status is 404
- **AND** the body does not equal `index.html`

#### Scenario: A request names a folder

- **GIVEN** a server running with `WEB_ROOT` set to a built web shell that holds an `assets` folder
- **AND** a tester has added the file `assets/index.html` to it
- **WHEN** a tester runs `curl -si http://localhost:5000/assets` and `curl -si http://localhost:5000/assets/`
- **THEN** both statuses are 404
- **AND** neither response has a `Location` header
- **AND** neither body names a file in that folder
