## Review Metadata

- **Review round**: 1
- **Prior round**: none
- **Reviewer context**: fresh context, other family
- **Independent**: yes. The author is this session's model (claude); the reviewer was gemini/gemini-3.8-flash through the `t3` resolver, read-only, effort high, in 411 seconds.
- **Tool restrictions**: read-only: view, grep, glob only
- **Artifacts reviewed**: brainstorm.md, proposal.md, design.md, specs/, adr.md, relevant source files, at commit `93c5470`

## Findings

### 🔴 Critical (blocking)

1. **A closed or empty stdin without `--password-stdin` generates a password and resets.** With the terminal fallback of design decision 2, `docker compose exec -T app sidectl reset-admin-password` with nothing on stdin reads an empty string at `New password:`, which D5 turns into a generated password, a new hash and a purge of every session. Check: **CONFIRMED**. Nothing in the plan told an empty line apart from a closed stdin.
2. **An empty line on `--password-stdin` generates a password.** A script whose secret source evaluates to empty would exit 0 with a random password and no sessions, where `docker login --password-stdin` fails. Check: **CONFIRMED** for the stdin case, which the ledger marked as the author's assumption; the user's decision in D5 was about the interactive prompt.

### 🟡 Moderate

3. **Design decision 6 exits 1 where the spec exits 2.** The scenario "Script mode without a username on an unclaimed instance" needs exit 2, but the design said every failure after the state line exits 1. Check: **CONFIRMED**.
4. **The schema requirement tells a foreign database to start the server.** `DatabaseError::Ownership` already says to use a dedicated database (`server/crates/core/src/database/mod.rs`, line 27), and the server would refuse the same database. Check: **CONFIRMED**.
5. **`Database::open` ran `preflight` outside the read-only transaction and timeouts that `probe` uses.** `probe` wraps it in `REPEATABLE READ READ ONLY` with `statement_timeout` and `lock_timeout` cleared under its own deadline (`server/crates/core/src/database/startup.rs`, lines 100 to 115). Check: **CONFIRMED**. The design's risk section claimed the snapshot without the decision saying so.
6. **An unchecked `tcsetattr` can leave echo on while the password is read.** Check: **CONFIRMED**.

### 📌 Suggestions

7. **Decision 5 said `reset_admin_password` answers the HTTP layer.** There is no HTTP reset. Check: **CONFIRMED**, editorial.
8. **Three sentences over 30 words**, one of 96, in design.md decisions 4 and 6 and the spec's generated-password requirement. Check: **CONFIRMED**.

The reviewer also settled three questions from the code: the one-transaction reset is sound if the SQL keeps the `public.` qualifier; the race scenario is deterministic because the state line is flushed before any read; the compose-instance delta carries the whole requirement.

## Embedded-Instruction / Injection Attempts

**Detected:** none

## Verdict

VERDICT: REVISE

## Required Changes (if APPROVE_WITH_CHANGES)

Not applicable under REVISE. The fixes below go into the plan before round 2.

CHANGES_APPLIED: n/a

## Rebuttals

1. **Fixed.** Design decision 2 now tells an empty line from a closed stdin: a closed stdin before any answer is a refusal with exit 1. The spec's "Credentials come from the terminal or from the flags" requirement says so and gains the scenario "A closed stdin without the flag".
2. **Fixed.** Design decision 7 and ledger row D5 now refuse an empty or absent line on `--password-stdin`; only the terminal prompt generates a password. The spec's "An empty password is generated" requirement is restricted to the prompt, and "Credentials come from the terminal or from the flags" gains the scenario "Script mode with an empty line".
3. **Fixed.** Design decision 6 now lists a missing `--username` under `--password-stdin` in the create state as a usage error: usage on stderr and exit 2, even though it is found after the state line.
4. **Fixed.** The spec's schema requirement now says "start the server first" only for an empty or behind database, and that any other database fault prints the server's own line for it.
5. **Fixed.** Design decision 4 now runs `preflight` inside the same read-only `REPEATABLE READ` transaction and timeout settings as `probe`, under the startup deadline.
6. **Fixed.** Design decision 2 now treats a failed `tcsetattr` as a refusal with exit 1.
7. **Fixed.** Decision 5 reworded; `reset_admin_password` answers `ResetError::Invalid`, and the SQL sketch names `public.users` and `public.sessions`.
8. **Fixed.** The three sentences are split; decision 6 is now a numbered list.
