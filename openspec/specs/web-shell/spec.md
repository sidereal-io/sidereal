# web-shell Specification

## Purpose

The web shell is Sidereal's user interface in `web/`. It installs and runs on its own, and shows live state from the server, starting with the server's health.

## Requirements

### Requirement: The web shell pins its pnpm release

`web/package.json` SHALL name one exact pnpm 12 release in its `packageManager` field. Any pnpm 12 release run in `web/` SHALL run the pinned release instead. `web/pnpm-lock.yaml` SHALL record the pinned release and its hashes.

#### Scenario: The pin names one exact release

- **WHEN** a reviewer reads the `packageManager` field of `web/package.json`
- **THEN** the value matches `pnpm@12.<minor>.<patch>`, where `<minor>` and `<patch>` are numbers

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

- **WHEN** the server is running and a user opens the web shell's home page
- **THEN** the page shows the text `healthy` within 5 seconds

#### Scenario: The server is not running

- **WHEN** nothing listens on the server's port and a user opens the web shell's home page
- **THEN** the page shows the text `unreachable` within 5 seconds

#### Scenario: The server answers with an unexpected body

- **WHEN** a stub on the server's port answers `GET /healthz` with status 200 and the body `{"status":"<b>degraded</b>"}`
- **AND** a user opens the web shell's home page
- **THEN** the page shows the text `unreachable`
- **AND** the page text does not contain `degraded`

#### Scenario: The server does not answer

- **WHEN** a stub on the server's port accepts connections to `GET /healthz` but never answers
- **AND** a user opens the web shell's home page
- **THEN** the page shows the text `unreachable` within 10 seconds

### Requirement: The home screen keeps checking while it is open

The home screen SHALL check the server's health again 5 seconds after each check finishes. It SHALL keep checking for as long as the page is open. It SHALL update the shown state after each check, with no reload.

#### Scenario: The server starts after the page opens

- **WHEN** the page shows `unreachable`, and the server then starts
- **THEN** the page shows `healthy` within 10 seconds of the server listening, without a reload

#### Scenario: The server stops while the page is open

- **WHEN** the page shows `healthy`, and the server then stops
- **THEN** the page shows `unreachable` within 10 seconds, without a reload

### Requirement: The web shell reaches the server through its own origin

In development, the web shell SHALL request the server's health at `/healthz` on its own origin. The web shell's dev server SHALL forward `/healthz` to the server at `http://localhost:5000`. The server therefore needs no cross-origin settings for the web shell.

#### Scenario: The browser asks the web shell's origin

- **WHEN** a user opens the web shell's home page at `http://localhost:5173`
- **THEN** every health request the browser sends goes to `http://localhost:5173/healthz`

#### Scenario: The dev server forwards to the server

- **WHEN** the server and the web shell are both running
- **AND** a tester runs `curl -s http://localhost:5173/healthz`
- **THEN** the output is `{"status":"ok"}`

### Requirement: The web shell's dev server allows no cross-origin reads

The web shell's dev server SHALL NOT send an `Access-Control-Allow-Origin` header on any response. This covers the responses it forwards from the server. A page from another origin therefore cannot read what the dev server serves.

#### Scenario: Another local origin asks for the server's health

- **WHEN** the server and the web shell are both running
- **AND** a tester runs `curl -si -H 'Origin: http://localhost:3000' http://localhost:5173/healthz`
- **THEN** the response headers contain no `Access-Control-Allow-Origin` header

#### Scenario: Another local origin sends a preflight request

- **WHEN** the web shell is running
- **AND** a tester runs `curl -si -X OPTIONS -H 'Origin: http://localhost:3000' -H 'Access-Control-Request-Method: GET' http://localhost:5173/healthz`
- **THEN** the response headers contain no `Access-Control-Allow-Origin` header

### Requirement: The web shell's dev server stays on this machine

The web shell's dev server SHALL listen on port 5173 and on loopback addresses only. When port 5173 is taken, the dev server SHALL stop with an error instead of listening on another port.

#### Scenario: A contributor starts the web shell

- **WHEN** a contributor starts the web shell with port 5173 free
- **AND** a tester lists listening TCP sockets with `ss -ltnp`
- **THEN** the web shell's dev server listens on port 5173
- **AND** every address it listens on is `127.0.0.1` or `[::1]`

#### Scenario: Port 5173 is taken

- **WHEN** another process listens on port 5173, and a contributor runs `just web`
- **THEN** the command exits with a non-zero status
- **AND** the web shell's dev server listens on no port

### Requirement: The health screen has unit tests

The web shell SHALL have unit tests for its health screen. The tests SHALL check that the screen shows `healthy` for status 200 with the body `{"status":"ok"}`. They SHALL also check that it shows `unreachable` for a status other than 200.

The tests SHALL run without a server, without a browser, and without network access. They SHALL replace the screen's health request with a stub.

#### Scenario: The tests run with no server

- **WHEN** nothing listens on port 5000, and a contributor runs `pnpm test` in `web/`
- **THEN** the command exits with status 0
- **AND** the test report lists at least two passing tests for the health screen

#### Scenario: The healthy check breaks

- **WHEN** a tester changes the health screen so that status 200 with `{"status":"ok"}` shows `unreachable`
- **AND** runs `pnpm test` in `web/`
- **THEN** the command exits with a non-zero status

#### Scenario: The unreachable check breaks

- **WHEN** a tester changes the health screen so that any status shows `healthy`
- **AND** runs `pnpm test` in `web/`
- **THEN** the command exits with a non-zero status

### Requirement: The web shell installs without a root npm project

The web shell SHALL live in `web/`, with its own `package.json` and `pnpm-lock.yaml`. Git SHALL track both files. The web shell SHALL install and run with no `package.json` or `package-lock.json` at the repo root. The repo root SHALL have neither file.

#### Scenario: The web shell installs from a fresh clone

- **WHEN** a contributor clones the repo and runs `pnpm --dir web install --frozen-lockfile`
- **THEN** the command exits with status 0
- **AND** the repo root has no `node_modules` folder

#### Scenario: The manifest and lockfile are tracked

- **WHEN** a reviewer runs `git ls-files web/package.json web/pnpm-lock.yaml`
- **THEN** the output lists both files

#### Scenario: The repo root has no npm project

- **WHEN** a reviewer runs `git ls-files package.json package-lock.json`
- **THEN** the output is empty

### Requirement: The web shell serves its own fonts

The web shell SHALL serve the font files for Inter and Atkinson Hyperlegible Mono from its own origin. It SHALL register each font under the family name that `DESIGN.md`'s font tokens put first: `Inter` and `Atkinson Hyperlegible Mono`. It SHALL NOT load a font, or anything else, from another origin.

#### Scenario: Inter loads from the web shell

- **WHEN** a user opens the web shell's home page
- **AND** `await document.fonts.ready` resolves in the browser console
- **THEN** `[...document.fonts].some(f => f.family === "Inter" && f.status === "loaded")` returns `true` in the browser console
- **AND** the browser's network log shows that the Inter font file came from the web shell's origin

#### Scenario: The mono font is registered

- **WHEN** a user opens the web shell's home page
- **THEN** `[...document.fonts].some(f => f.family === "Atkinson Hyperlegible Mono")` returns `true` in the browser console

#### Scenario: Nothing loads from another origin

- **WHEN** a user opens the web shell's home page
- **AND** `await document.fonts.ready` resolves in the browser console
- **THEN** the browser's network log shows no request to any origin other than the web shell's

### Requirement: The health screen uses the design tokens

The health screen SHALL take its page background, its text color, and its font family from the token file, not from values written in its own styles. It SHALL use `obsidian-950` for the page background, `obsidian-100` for text, and `font-family-sans` for the font.

#### Scenario: The screen shows the token values

- **WHEN** a user opens the web shell's home page
- **THEN** `getComputedStyle(document.body).backgroundColor` returns `rgb(11, 10, 15)`
- **AND** `getComputedStyle(document.body).color` returns `rgb(245, 244, 239)`
- **AND** `getComputedStyle(document.body).fontFamily` starts with `Inter`

#### Scenario: A token change reaches the screen

- **WHEN** a tester changes `obsidian-950` to `#200000` in `DESIGN.md`, runs the generator, and reloads the home page
- **THEN** `getComputedStyle(document.body).backgroundColor` returns `rgb(32, 0, 0)`
