## MODIFIED Requirements

### Requirement: One command runs the server and the web shell

`just dev` SHALL start the server and the web shell together against a prepared database configured through `DATABASE_URL`. One interrupt, such as Ctrl-C, SHALL stop both. When either one exits, `just dev` SHALL stop the other and exit. It SHALL exit with a non-zero status when either one failed. `just dev` SHALL NOT clear the terminal, so earlier output, such as a cargo error, stays visible.

#### Scenario: A fresh clone runs the stack

- **WHEN** a contributor clones the repo and prepares PostgreSQL using the documented root fixture recipe
- **AND** configures `DATABASE_URL` and runs `just dev` inside the development shell
- **AND** opens `http://localhost:5173` once the server has built
- **THEN** the page shows `healthy`
- **AND** the repo root still has no `node_modules` folder

#### Scenario: Ctrl-C stops the whole stack

- **GIVEN** a prepared compatible database and configured `DATABASE_URL`
- **WHEN** `just dev` is running and the contributor presses Ctrl-C
- **THEN** `just` exits
- **AND** within 5 seconds nothing listens on port 5000 or port 5173

#### Scenario: The terminal keeps earlier output

- **GIVEN** a prepared compatible database and configured `DATABASE_URL`
- **WHEN** a tester runs `script -qc 'just dev' dev.log` inside the development shell on Linux
- **AND** stops it with Ctrl-C after the web shell's dev server listens on port 5173
- **THEN** `dev.log` contains none of these escape sequences: `ESC [ 2 J`, `ESC [ 0 J`, `ESC [ J`, or `ESC c`

#### Scenario: The web shell fails to start

- **GIVEN** a prepared compatible database and configured `DATABASE_URL`
- **WHEN** another process listens on port 5173, and a contributor runs `just dev`
- **THEN** `just dev` exits with a non-zero status within 10 seconds
- **AND** nothing that `just dev` started is still running


## ADDED Requirements

### Requirement: Root recipes report fixture connection settings

`just db-url` SHALL report the runtime fixture connection URL for the selected worktree project and loopback port.
`just db-test-url` SHALL report its separate test-role URL.
Both recipes SHALL use only fixture credentials and SHALL NOT read or print production `DATABASE_URL`.
Operator guidance SHALL show exporting these values before running development and test commands.

#### Scenario: Fixture URLs match the selected port
- **WHEN** a contributor selects a fixture port and runs `just db-url` and `just db-test-url`
- **THEN** both URLs use that port, their distinct documented roles, and explicit `sslmode=disable`

#### Scenario: Production configuration stays private
- **WHEN** a contributor runs either URL recipe with a distinctive production `DATABASE_URL` in the environment
- **THEN** the output contains only the fixture URL and no part of that production connection string
