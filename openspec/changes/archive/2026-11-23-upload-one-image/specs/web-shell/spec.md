## MODIFIED Requirements

### Requirement: The web shell reaches the server through its own origin

In development, the web shell SHALL request the server's health at `/healthz` and the access routes and the events socket under `/api` on its own origin. Its dev server SHALL forward `/healthz` and `/api`, WebSocket upgrades included, to `http://localhost:5000`, or to `SIDEREAL_DEV_PROXY_TARGET` when the end-to-end runner sets it, and SHALL keep the browser's `Host` header. The server needs no cross-origin settings for the web shell.

#### Scenario: The browser asks the web shell's origin

- **WHEN** a user opens the web shell's home page at `http://localhost:5173`
- **THEN** every health and access request the browser sends goes to `http://localhost:5173`

#### Scenario: The dev server forwards to the server

- **WHEN** the server and the web shell are both running
- **AND** a tester runs `curl -s http://localhost:5173/healthz`
- **THEN** the output is `{"status":"ok"}`

#### Scenario: The dev server forwards the access routes

- **WHEN** the server and the web shell are both running against an unclaimed database
- **AND** a tester runs `curl -s http://localhost:5173/api/claim`
- **THEN** the output is `{"open":true}`

#### Scenario: The dev server forwards the events socket

- **WHEN** the server and the web shell are both running and Sam is signed in at `http://localhost:5173`
- **AND** the page opens a WebSocket to `ws://localhost:5173/api/events`
- **THEN** the upgrade answers 101 and the socket receives `{"type":"heartbeat"}` within 35 seconds

#### Scenario: The end-to-end runner names another server

- **GIVEN** nothing listens on port 5000
- **WHEN** a tester starts the server with `PORT=5901` and the web shell with `SIDEREAL_DEV_PROXY_TARGET=http://localhost:5901` and `--port 5902`
- **AND** runs `curl -s http://localhost:5902/healthz`
- **THEN** the output is `{"status":"ok"}`

### Requirement: The web shell's dev server stays on this machine

The web shell's dev server SHALL listen on loopback addresses only. It SHALL listen on port 5173 unless its command names another port; only the end-to-end runner names one. When its port is taken, the dev server SHALL stop with an error instead of listening on another port.

#### Scenario: A contributor starts the web shell

- **WHEN** a contributor starts the web shell with port 5173 free
- **AND** a tester lists listening TCP sockets with `ss -ltnp`
- **THEN** the web shell's dev server listens on port 5173
- **AND** every address it listens on is `127.0.0.1` or `[::1]`

#### Scenario: Port 5173 is taken

- **WHEN** another process listens on port 5173, and a contributor runs `mise run web`
- **THEN** the command exits with a non-zero status
- **AND** the web shell's dev server listens on no port

### Requirement: The access section uses the design tokens

The access section SHALL take every color, size, radius and font from the token file, using a semantic token wherever one exists for the role. The submit button SHALL use `color-action-primary` with a `color-text-on-action` label and `radius-4`. Each field SHALL use `radius-4`, a `border-width-1` border in `color-border-hairline`, `obsidian-900` as its background, and `color-text-body` text.

#### Scenario: The submit button shows the token values

- **WHEN** the page shows the claim or sign-in form
- **THEN** the submit button's computed background color is `rgb(124, 58, 237)` and its border radius is `4px`

#### Scenario: The access stylesheet uses the semantic tokens

- **WHEN** a reviewer runs `grep -cE 'var\(--sr-(violet-600|white|obsidian-100|obsidian-400|obsidian-800)\)' web/src/Access.css`
- **THEN** the output is `0`
