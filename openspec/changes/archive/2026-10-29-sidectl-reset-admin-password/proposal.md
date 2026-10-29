## Why

The instance claim is open to the first visitor, so a stranger who arrives first can take Sam's instance. There is no mail to recover a forgotten password. A command on the box is the only recovery path. It is also the zero-exposure way to claim before anyone else can.

## What Changes

- A second binary, `sidectl`, in the server crate, copied into the app image beside `sidereal-server` and run with `docker compose exec app sidectl …` (D12).
- One command, `sidectl reset-admin-password`, with two states decided by the database. When no administrator exists it creates one, which closes the claim. Otherwise it sets a new password for the administrator and ends every session, in one transaction (D8). It announces its state before prompting (D6) and prints one line saying what it did and for whom (D10).
- Interactive prompts with hidden input, the password asked twice (D3), and a script mode selected by `--password-stdin`, with `--username` for the create state (D2). An empty password generates one and prints it once (D5). A `--username` that does not name the existing administrator is refused (D4).
- The claim's credential rules apply (D7). The command never migrates: it checks the schema is current and refuses otherwise (D1). It connects with the server's database settings (D11).
- A `Recover access` section in `docs/install.md` (D14).
- Integration tests for both states, for sessions being gone after a reset, and for each refusal (D15).

Nothing is removed and nothing breaks.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `instance-access`: new requirements for the `sidectl reset-admin-password` command: its two states, its prompts and script mode, the generated password, its refusals, and its output (D13).
- `compose-instance`: the requirement "The app image serves the web shell and the health probes on one origin" says the image holds the server binary; it now also holds `sidectl`, runnable through `docker compose exec app`.

## Impact

- `server/crates/server/Cargo.toml`: a second `[[bin]]`, and a dependency for reading a password without echo.
- `server/crates/server/src/`: the `sidectl` binary, its argument parsing, prompts and output.
- `server/crates/core/src/access/` and `database/`: a reset method that changes the hash and deletes every session in one transaction, and a way to connect that checks the schema without migrating.
- `server/crates/server/tests/`: integration tests that spawn `sidectl`.
- `Dockerfile`: the runtime stage copies `sidectl`.
- `docs/install.md`: the `Recover access` section.
- `openspec/specs/instance-access/spec.md` and `openspec/specs/compose-instance/spec.md`.
