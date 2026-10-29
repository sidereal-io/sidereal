## Story

#449 sidectl-reset-admin-password — create the administrator or reset the password from the box
https://github.com/sidereal-io/sidereal/issues/449

## Decisions

| # | Question | Answer | Source |
|---|---|---|---|
| D1 | Does `siderealctl` run the server's migrations? | No. It checks that the schema is current, as `/readyz` does, and exits with a message when the server has not migrated yet. Only the server changes the schema. | User |
| D2 | How does a script select the non-interactive mode? | An explicit `--password-stdin` flag, as `docker login` has: it reads one line from stdin as the password. In the create state the username comes from `--username <u>`. Without the flag the command prompts. Scripts run `docker compose exec -T app siderealctl reset-admin-password --username sam --password-stdin`. | User |
| D3 | Is the interactive password asked twice? | Yes, with hidden input: `New password:` then `Repeat password:`; a mismatch is refused before any database change. `--password-stdin` reads one line and never repeats. Immich asks once, in plain text. | User |
| D4 | What if `--username` is given when an administrator exists? | It must name that administrator, otherwise the command refuses before any change. The success line always names the administrator. | User |
| D5 | What does an empty password do? | At the terminal prompt, as Immich: an empty answer to `New password:` makes the command generate a random 24-character password, print it once on stdout, skip the repeat, and use it. This is the one time a password appears in output. On `--password-stdin` an empty or missing line is refused: a script must be explicit. A stdin that closes before an answer is refused in either mode. | User; stdin and closed-stdin cases: Assumption, revised after review round 1 |
| D6 | Does the command say which state it is in before prompting? | Yes. Before any prompt it prints `No administrator exists. Creating one.` or `Administrator <name> exists. Setting a new password and ending every session.` | User |
| D7 | Which credential rules apply? | The claim's: a username of 1 to 64 characters with no surrounding whitespace, a password of 8 to 256 bytes. A violation is refused before any change. | Evidence: `server/crates/core/src/access/claim.rs` |
| D8 | What does the reset change? | In one transaction: the administrator's password hash, and every row in `sessions`. The guard reads the session row on every request, so each signed-in browser is refused on its next request, whether or not the server is running. | Evidence: `server/crates/server/src/access/guard.rs`, `openspec/glossary.md` (Access recovery) |
| D9 | What if an administrator appears between the announcement and the create? | The unique index refuses the insert; the command reports that an administrator now exists and exits non-zero. Running it again takes the reset path. | Assumption |
| D10 | What does the command print, and with which exit code? | One success line on stdout naming what it did and for whom, 0. Every refusal is one line on stderr naming the cause, never a password, a hash or a URL, 1. A usage error, such as no subcommand or an unknown flag, prints the usage on stderr, 2. | Assumption |
| D11 | How does `siderealctl` reach PostgreSQL? | Through `DatabaseConfig::from_env`: the same `DATABASE_URL`, timeouts, TLS mode and certificate settings as the server, so it works unchanged inside `docker compose exec app`. | Evidence: `server/crates/server/src/config.rs` |
| D12 | How does the binary get into the image? | `cargo build --release -p sidereal-server` builds every `[[bin]]` of the package; the runtime stage copies `siderealctl` beside `sidereal-server` into `/usr/local/bin`. It runs as the `app` user with capabilities dropped, which it does not need. | Evidence: `Dockerfile`, `compose.app.yml` |
| D13 | Where do the requirements live? | In the existing `instance-access` capability, as new requirements. Access recovery is part of that capability's purpose. | Assumption |
| D14 | Which guide section changes? | Only a new `Recover access` section in `docs/install.md`. #450 owns replacing `The app has no sign-in yet` with `Claim your instance`, which is where the "run `siderealctl` right after the first start" advice goes. | Evidence: issue #450 scope |
| D15 | What tests does this story carry? | Integration tests that spawn `siderealctl` with `--password-stdin` against a fresh database: the create state closes the claim and the new administrator can sign in; the reset state changes the password and leaves no session; a username mismatch, a short password and an unmigrated database change nothing. The interactive prompts, including the generated password, are tested in-process through a terminal seam with a fake terminal against the same fresh database, not through a real terminal. | Evidence: issue #449 scope; seams: Assumption, revised after review round 3 |
| D16 | What is the command called? | `siderealctl`, renamed from `sidectl` for clarity. The change and its branch keep the name `sidectl-reset-admin-password`. | User |

## Changes from the story

- **Added**: an empty password generates and prints a random one (D5), borrowed from Immich. The scope line "never echoes the password" now means: never echoes a password it was given.
- **Added**: the command announces its state before prompting (D6).
- **Added**: `--username` and `--password-stdin` as the script interface (D2), and the repeated prompt (D3). The story named a non-interactive mode without saying how it is selected.
- **Cut**: nothing. **Split out**: nothing.

Confirmed by user: yes
