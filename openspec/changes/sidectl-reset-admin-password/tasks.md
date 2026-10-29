## 1. Core contracts

- [x] 1.1 [medium] `Database::open` returns a handle when the schema is current, answers `DatabaseError::Pending` for an empty or behind database, passes other preflight errors through, and creates no table (verify: cargo test -p sidereal-core --manifest-path server/Cargo.toml open_) (✓ cargo test -p sidereal-core open_ passed)
- [x] 1.2 [easy] [parallel] `check_username` and `check_password` name the rule broken, and `claim` still answers `Invalid` through them (verify: cargo test -p sidereal-core --manifest-path server/Cargo.toml credential_rule) (✓ cargo test -p sidereal-core credential_rule passed; claim tests green)
- [x] 1.3 [medium] `Database::administrator` names the administrator, and `reset_admin_password` replaces the hash and deletes every session in one transaction, answering `NoAdministrator`, `Invalid` and `Unavailable` as designed (verify: cargo test -p sidereal-core --manifest-path server/Cargo.toml reset_admin_password) (✓ cargo test -p sidereal-core reset_admin_password and administrator passed)
- [x] 1.4 [easy] [parallel] `generate_password` returns 24 ASCII letters and digits and differs between calls (verify: cargo test -p sidereal-core --manifest-path server/Cargo.toml generate_password) (✓ cargo test -p sidereal-core generate_password passed)

## 2. The sidectl binary [depends-on: 1]

- [x] 2.1 [easy] The argument parser answers `Help`, the command with its options, or a usage error: no command, an unknown command or flag, `--username` without a value; `--username ""` parses as an empty string (verify: cargo test -p sidereal-server --manifest-path server/Cargo.toml sidectl::args) (✓ cargo test -p sidereal-server sidectl::args passed)
- [x] 2.2 [medium] The prompt sequence against a fake terminal: `Username:` only in the create state and only without `--username`, the password rule checked before `Repeat password:`, a mismatch refused, an empty answer generated only when the terminal is interactive, a closed stdin refused with its own message, prompts on stderr (verify: cargo test -p sidereal-server --manifest-path server/Cargo.toml sidectl::prompts) (✓ cargo test -p sidereal-server sidectl::prompts passed; prompts go through `Terminal::prompt`, which 2.3 binds to stderr)
- [x] 2.3 [medium] The real terminal and the binary: a hidden read through termios whose guard restores echo on drop, the `ENOTTY` fallback to a plain read, `--password-stdin` reading one line without its ending, and a second `[[bin]]` with `default-run` so `cargo run -p sidereal-server` still runs the server (verify: cargo test -p sidereal-server --manifest-path server/Cargo.toml sidectl::terminal) (✓ cargo test -p sidereal-server sidectl::terminal passed; `cargo run -p sidereal-server` still starts the server)
- [x] 2.4 [medium] `sidectl::run` in-process with a fake interactive terminal against a fresh database: the state line first, the create state creates an administrator who can sign in, the reset state reports the session count and a deleted session's cookie is refused, a wrong `--username` is refused, and a generated password is printed only after success and signs in (verify: cargo test -p sidereal-server --manifest-path server/Cargo.toml --test sidectl in_process) (✓ cargo test -p sidereal-server --test sidectl in_process passed, 8 tests)

## 3. Image and guide [depends-on: 2]

- [x] 3.1 [easy] [parallel] The runtime image holds `sidectl` beside the server, and `instance-verify.sh` checks `docker compose exec -T app sidectl --help` exits 0 and names `reset-admin-password` (verify: just instance-verify) (✓ just instance-verify passed)
- [x] 3.2 [easy] [parallel] `docs/install.md` gains a `Recover access` section before `Things to know`, as design decision 9 lists, in plain language (verify: grep -q '^## Recover access' docs/install.md) (✓ grep found `## Recover access` before `## Things to know`)

## 4. Integration [depends-on: 3]

- [x] 4.1 [complex] The spawned binary against a fresh database covers every remaining scenario of the instance-access delta: both states through `--password-stdin`, sessions gone, the line ending stripped, the wrong and the right `--username`, a short password and a padded username, an empty and a closed stdin in both modes, the race, an empty database, a foreign database, a missing `DATABASE_URL`, secrets absent from output, help and an unknown command (verify: cargo test -p sidereal-server --manifest-path server/Cargo.toml --test sidectl) (✓ cargo test -p sidereal-server --test sidectl passed, 34 tests)
- [x] 4.2 [easy] The whole gate is green, including the arch lint and the web checks (verify: just check) (✓ just check passed: server, arch lint, compose and web)

## Deviations Ledger

<!-- Dated notes on small departures from the plan, written during apply. Stop and ask the user about a large one. -->

- 2026-10-29, task 2.1: `--username` given twice is a usage error, exit 2. The plan did not say which value wins; refusing avoids guessing whose account a script meant.
