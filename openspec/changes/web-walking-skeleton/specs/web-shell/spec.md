## Purpose

The web shell is the v2 web frontend in `web/`. It is a standalone app that shows live state from the v2 server, starting with the server's health.

## ADDED Requirements

### Requirement: The web shell is a standalone app

The web shell SHALL live in `web/`, with its own `package.json` and `pnpm-lock.yaml`. Git SHALL track both files. The web shell SHALL install and run without the root npm workspace. The root `package.json` and `package-lock.json` SHALL NOT refer to `web/`.

#### Scenario: The web shell installs without the v0.10.x tree

- **WHEN** a contributor clones the repo and runs `pnpm --dir web install --frozen-lockfile`, without running `npm install` first
- **THEN** the command exits with status 0
- **AND** the repo root has no `node_modules` folder

#### Scenario: The manifest and lockfile are tracked

- **WHEN** a reviewer runs `git ls-files web/package.json web/pnpm-lock.yaml`
- **THEN** the output lists both files

#### Scenario: The v0.10.x tree does not refer to the web shell

- **WHEN** a reviewer runs `grep -n '"web/' package.json package-lock.json`
- **THEN** the search finds no match

### Requirement: The web shell pins its pnpm release

`web/package.json` SHALL name one exact pnpm 12 release in its `packageManager` field. Any pnpm 12 release run in `web/` SHALL run the pinned release instead. `web/pnpm-lock.yaml` SHALL record the pinned release and its hashes.

#### Scenario: The pin names one exact release

- **WHEN** a reviewer reads the `packageManager` field of `web/package.json`
- **THEN** the value matches `pnpm@12.<minor>.<patch>`, with both numbers given

#### Scenario: Another pnpm 12 release runs the pinned one

- **WHEN** a contributor runs `pnpm --version` in `web/` with a pnpm 12 release other than the pinned one on `PATH`
- **THEN** the output is the release that `packageManager` names

#### Scenario: The lockfile records the pinned release

- **WHEN** a reviewer reads `web/pnpm-lock.yaml`
- **THEN** it lists `pnpm` under `packageManagerDependencies`, at the release that `packageManager` names

### Requirement: pnpm verifies the pinned release before running it

pnpm SHALL refuse to run the pinned release when its download does not match the hashes in `web/pnpm-lock.yaml`.

#### Scenario: The lockfile hash does not match the download

- **WHEN** a tester changes the hash recorded for the pinned `pnpm` package in `web/pnpm-lock.yaml`
- **AND** runs `pnpm --version` in `web/` with an empty pnpm cache
- **THEN** the command exits with a non-zero status
- **AND** it prints no version number

### Requirement: The home screen shows the server's health

The web shell's home screen SHALL show exactly one of three health states:

- **checking**, before the first health check finishes;
- **healthy**, when the last check got status 200 and a JSON body whose `status` field is `"ok"`;
- **unreachable**, when the last check got any other result, including a network error, another status, a body that is not that JSON, or no answer within 5 seconds.

The screen SHALL show the state's name as visible text. The screen SHALL NOT show any text taken from the server's response.

#### Scenario: The server is running

- **WHEN** the v2 server is running and a user opens the web shell's home page
- **THEN** the page shows the text `healthy` within 5 seconds

#### Scenario: The server is not running

- **WHEN** nothing listens on the v2 server's port and a user opens the web shell's home page
- **THEN** the page shows the text `unreachable` within 5 seconds

#### Scenario: The server answers with an unexpected body

- **WHEN** a stub on the v2 server's port answers `GET /healthz` with status 200 and the body `{"status":"<b>degraded</b>"}`
- **AND** a user opens the web shell's home page
- **THEN** the page shows the text `unreachable`
- **AND** the page text does not contain `degraded`

#### Scenario: The server does not answer

- **WHEN** a stub on the v2 server's port accepts connections to `GET /healthz` but never answers
- **AND** a user opens the web shell's home page
- **THEN** the page shows the text `unreachable` within 10 seconds

### Requirement: The home screen keeps checking while it is open

The home screen SHALL check the server's health again 5 seconds after each check finishes. It SHALL keep checking for as long as the page is open. It SHALL update the shown state after each check, with no reload.

#### Scenario: The server starts after the page opens

- **WHEN** the page shows `unreachable`, and the v2 server then starts
- **THEN** the page shows `healthy` within 10 seconds of the server listening, without a reload

#### Scenario: The server stops while the page is open

- **WHEN** the page shows `healthy`, and the v2 server then stops
- **THEN** the page shows `unreachable` within 10 seconds, without a reload

### Requirement: The web shell reaches the server through its own origin

In development, the web shell SHALL request the server's health at `/healthz` on its own origin. The web shell's dev server SHALL forward `/healthz` to the v2 server at `http://localhost:5000`. The v2 server therefore needs no cross-origin settings for the web shell.

#### Scenario: The browser asks the web shell's origin

- **WHEN** a user opens the web shell's home page at `http://localhost:5173`
- **THEN** every health request the browser sends goes to `http://localhost:5173/healthz`

#### Scenario: The dev server forwards to the v2 server

- **WHEN** the v2 server and the web shell are both running
- **AND** a tester runs `curl -s http://localhost:5173/healthz`
- **THEN** the output is `{"status":"ok"}`

### Requirement: The web shell's dev server stays on this machine

The web shell's dev server SHALL listen on port 5173 when that port is free. It SHALL listen on loopback addresses only.

#### Scenario: A contributor starts the web shell

- **WHEN** a contributor starts the web shell with port 5173 free
- **AND** a tester lists listening TCP sockets with `ss -ltnp`
- **THEN** the web shell's dev server listens on port 5173
- **AND** every address it listens on is `127.0.0.1` or `[::1]`
