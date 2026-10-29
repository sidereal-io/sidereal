## Context

- `sidereal-core::access` already holds the credential rules, Argon2id hashing on the blocking pool, the claim with its race decided by the `users_one_admin` index, and the session queries, all as methods on `Database`. Its module comment says it is reachable without HTTP so `sidectl` can share it.
- `Database` has one constructor path: `initialize` and `initialize_with_storage` migrate under the startup lock, then build the pools. `Database::new` is private. The readiness probe already knows how to check the live schema without migrating: `preflight` on a direct connection, expecting every migration applied.
- `sidereal-server` has one `[[bin]]`, `main.rs`, which reads the environment through `config.rs` (`DatabaseConfig::from_env`, timeouts, TLS mode, certificate files) and never retains a URL in an error.
- `DatabaseError` is a `Copy` enum with a fixed label and a fixed action per variant; the install guide's troubleshoot table lists the labels the server can print.
- The `Dockerfile` builds with `cargo build --release -p sidereal-server`, which compiles every binary target of the package, and copies one binary into `/usr/local/bin`. The container runs as `app` with every capability dropped.
- Integration tests in `crates/server/tests/` take a fresh database per test from `TestDatabase`, drive the router with `oneshot`, and spawn binaries through `CARGO_BIN_EXE_<name>`, which Cargo sets for every binary of the package.
- `libc` is already a workspace dependency, used by `core`. The server crate has no terminal or argument-parsing dependency.
- Constraints: `core` stays domain-agnostic; `packs/astro` is untouched; `just check` stays green; the guide follows plain language.

## Goals / Non-Goals

**Goals:**

- One binary, one command, whose every database effect is a `core` method that the HTTP routes could also call.
- The command reads the state, prompts, validates and only then writes, so a refusal never leaves a half-done change.
- The prompt logic is testable without a terminal, and the whole binary is testable through pipes.
- No new crate.

**Non-Goals:**

- A general CLI framework, subcommand discovery or shell completion.
- Audit rows for the reset (#453), rate limiting, or any second `sidectl` command.
- Changing how the server starts or migrates.
- Windows terminals: the binary ships in a Linux image.

## Test Seams

- **Primary test seam:** the spawned `sidectl` binary, through `CARGO_BIN_EXE_sidectl`, with stdin, stdout and stderr piped, against a `TestDatabase`, in a new `crates/server/tests/sidectl.rs`. The same test checks the effect through the in-process router (`app_with_database` with `oneshot`), which reads the database on every request: the claim state, sign-in with the old and the new password, and a cookie whose session the reset deleted. Both states, the generated password, every refusal, help and usage errors run here. The race runs here too: the binary prints its state line and waits at the username prompt while the test claims through the router, then the test answers the prompts.
- **Existing seams reused:** `core`'s own `#[tokio::test]` modules against `TestDatabase` for the new `Database` methods and for opening a database without migrating; unit tests in the server crate's `sidectl` module with an in-memory terminal for the prompt sequence and the argument parser; `scripts/instance-verify.sh` for the image scenario.

## Scrutiny Tier & Blast Radius

- **Tier:** high
- **Blast radius:** `core::access` gains a password reset that deletes every session and a public credential-rule check; `core::database` gains a second way to construct `Database` and one `DatabaseError` variant; the server crate gains a binary and a module; the `Dockerfile` copies a second binary into the image; `docs/install.md` gains a section; `instance-verify.sh` gains a check. The command creates the administrator and revokes every session, so it is a trust boundary of its own.

## Decisions

1. **The binary is `crates/server/src/bin/sidectl.rs`, a thin `main` over a `sidereal_server::sidectl` module.** The module holds the argument parser, the prompt sequence and the command, written against a `Terminal` trait with three operations: write a line, read an echoed line, read a hidden line. `main` supplies the real terminal and maps the result to an exit code. The `Cargo.toml` names both `[[bin]]`s and sets `default-run = "sidereal-server"`, so `cargo run -p sidereal-server` and the `justfile`'s `server` recipe keep working. Alternative: a new workspace crate. Rejected: the story puts it in the server crate, and the binary reuses `config.rs` as is.

2. **Hidden input through `libc` termios, with a guard that restores echo.** `read_hidden` calls `tcgetattr` on stdin, clears `ECHO`, reads a line, and restores the saved attributes in a `Drop` guard, so a panic or an early return leaves the terminal usable. When `tcgetattr` fails because stdin is not a terminal, the read falls back to a plain line read: there is no echo to turn off, and this is what lets the integration tests drive the prompts through a pipe. The prompt text is written to stdout and flushed before each read. Alternative: the `rpassword` crate. Rejected: it does the same `tcgetattr` dance behind a new dependency, and the fallback would need wrapping anyway.

3. **Arguments are parsed by hand from `std::env::args`.** The grammar is `sidectl [--help] <command> [--username <u>] [--password-stdin] [--help]`, with one command, `reset-admin-password`. A parse produces `Help`, a `Command` with its options, or a `Usage(String)` error that `main` prints to stderr with the usage text before exiting 2. `--username` with an empty value is a usage error. Alternative: `clap`. Rejected: three flags, and a new dependency with derive macros for them.

4. **`Database::open` connects and checks the schema without migrating.** A new constructor in `core::database::startup` opens one direct connection within the startup timeout, runs the existing `preflight`, and requires it to report every migration applied, as the readiness probe does; then it builds the pools with the private `Database::new` and closes the direct connection. It takes no lock and runs no migration. An empty database (`preflight` reports none applied) or a partial history maps to a new `DatabaseError::Pending`, whose label is `database schema is not current` and whose action is `start the server once to apply migrations`. The other preflight errors pass through unchanged, so a wrong URL, a foreign database or an unsupported version reads the same as in the server's log. Alternative: have `sidectl` call `initialize`. Rejected by D1. Alternative: a `sqlx::query` against `_sqlx_migrations` in the binary. Rejected: `core` owns the baselines and the ownership checks, and duplicating them would drift.

5. **`core::access` gains three things: the administrator lookup, the reset, and public credential checks.**
   - `Database::administrator() -> Result<Option<String>, Unavailable>` returns the administrator's username. It is the state check (D6) and the username check (D4). `claim_open` stays as it is for the HTTP route.
   - `Database::reset_admin_password(password) -> Result<Reset, ResetError>` hashes first, on the blocking pool, then runs one transaction: `UPDATE users SET password_hash = $1 WHERE role = 'admin' RETURNING username`, then `DELETE FROM sessions`, then commit. `Reset` carries the username and the number of sessions deleted. An update that touches no row rolls back and answers `ResetError::NoAdministrator`; a driver error answers `ResetError::Unavailable`. The transaction is what makes D8 true: the new hash and the empty table appear together or not at all.
   - `follows_length_rules` splits into `check_username(&str) -> Result<(), CredentialRule>` and `check_password(&str) -> Result<(), CredentialRule>`, public, where `CredentialRule` names the rule broken (`UsernameLength`, `UsernameWhitespace`, `PasswordLength`) and implements `Display` with the rule's text. `claim` and `reset_admin_password` keep calling them and still answer `Invalid` to the HTTP layer; `sidectl` calls them before any prompt for the password is repeated, so its stderr can name the rule, as the spec asks.
   - `generate_password()` returns 24 characters chosen uniformly from the 62 ASCII letters and digits with `rand`'s secure generator, which `core` already uses for tokens. It lives in `core` because `rand` is a `core` dependency and the server crate has none.

6. **The command runs in a fixed order, and writes last.** Parse arguments; read the database configuration; `Database::open`; `administrator()`; print the state line; resolve the username (flag, or prompt in the create state; in the reset state a flag that differs from the administrator is refused here); check the username; read the password (stdin line, or prompt); an empty password becomes a generated one and skips the repeat; otherwise prompt the repeat and refuse a mismatch; check the password; print `Generated password:` when one was generated; then one `core` call, `claim` or `reset_admin_password`; then the success line. `ClaimError::Claimed` from `claim` is the race (D9) and prints `an administrator now exists; run the command again to set its password`. Every failure after the state line is one stderr line and exit 1; nothing in the module formats a password, a hash or a URL, and the `DatabaseError` and `Unavailable` values it prints carry none.

7. **`--password-stdin` reads the first line only and strips one line ending.** `\n` or `\r\n` is removed once; nothing else is trimmed, so a password with a leading space stays as typed. An absent line, from an empty stdin, counts as an empty password and so generates one. Alternative: read the whole of stdin. Rejected: `docker login` reads one line, and a file with a trailing newline is the common case.

8. **The image copies the binary beside the server.** One `COPY --from=server /build/target/release/sidectl /usr/local/bin/sidectl` line in the runtime stage. Nothing else in the image changes: the binary needs the same environment, user and network as the server, and no capability.

9. **The guide gets a `Recover access` section before `Things to know`.** It says when to run the command, shows the interactive form and the script form with `docker compose exec` and `-T`, explains the two outcomes and the generated password, says that the reset signs out every browser, and names the `database schema is not current` message with its fix. The troubleshoot table is unchanged: that message never comes from the server.

10. **`instance-verify.sh` runs `docker compose exec -T app sidectl --help`.** One step next to the existing `exec` checks, asserting exit 0 and `reset-admin-password` in the output, so the image scenario has a test.

## Risks / Trade-offs

- [Echo stays off after a crash between `tcsetattr` and the guard's drop] → the guard is created in the same statement that clears echo, and the read happens inside its scope; a test checks that the guard restores the saved attributes on drop.
- [A stdin that is a terminal but `tcgetattr` fails for another reason reads the password in the clear] → the fallback is used only when `tcgetattr` reports `ENOTTY`; any other error is a refusal with exit 1.
- [The reset leaves a browser with a cookie the server has forgotten] → the guard clears a cookie that matches no row on the next request, as the `A refused session clears the stale cookie` requirement says; the guide says every browser is signed out.
- [`Database::open` races a server that is migrating] → the server holds the startup lock while migrating, and `open` takes none; it reads a consistent snapshot in a read-only transaction, as `probe` does, and reports `Pending` until the migration commits. The guide says to start the server first.
- [A second `DatabaseError` variant is read by the server's `match` arms] → the variant is matched explicitly where `DatabaseError` is matched exhaustively; `clippy -D warnings` catches a missed arm.
- [The generated password is printed on a terminal that is being recorded or scrolled back] → it is what the user chose over Immich's behavior (D5); the guide says to change it afterwards if the terminal is shared.
- [The integration test drives interactive prompts through a pipe, which is not a terminal] → the fallback in decision 2 is the designed behavior for a pipe, not a test-only path, and the unit tests cover the sequence with a fake terminal in both modes.

## Migration & Rollback

N/A. No migration and no schema change. Rolling back is deploying the previous image, which lacks the binary; the database is unaffected either way.
