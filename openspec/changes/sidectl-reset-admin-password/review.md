## Review Metadata

- **Review round**: 2
- **Prior round**: round 1 at commit `93c5470`, by gemini/gemini-3.8-flash, gave REVISE: a closed or empty stdin generated a password and reset the administrator in both input modes, plus an exit-code contradiction, a wrong message for a foreign database, `Database::open` outside `probe`'s transaction, an unchecked `tcsetattr` and long sentences. All eight were fixed before this round.
- **Reviewer context**: fresh context, other family
- **Independent**: yes. The author is this session's model (claude); the reviewer was gemini/gemini-3.8-flash through the `t3` resolver, read-only, effort high, in 626 seconds.
- **Tool restrictions**: read-only: view, grep, glob only
- **Artifacts reviewed**: brainstorm.md, proposal.md, design.md, specs/, adr.md, relevant source files, at commit `5df6836`

## Findings

### 🔴 Critical (blocking)

1. **An empty piped line without `--password-stdin` still generates a password and purges every session.** `echo "$SECRET" | sidectl reset-admin-password` with an empty `$SECRET` sends one line feed; under the `ENOTTY` fallback that is an empty answer at `New password:`, which design decision 6 turned into a generated password. Round 1's fix covered only `--password-stdin` and a closed stdin. Check: **CONFIRMED**.
2. **The prompt on stdout contradicts the scenario "A closed stdin without the flag".** Decision 2 flushed `New password:` to stdout before the read, so stdout could never hold only the state line. Check: **CONFIRMED**.

### 🟡 Moderate

3. **Decision 5 checked the password before the repeat prompt; decision 6 checked it after.** A four-character password mistyped at the repeat would report a mismatch instead of the length rule. Check: **CONFIRMED**.
4. **The `Pending` action text lacked the words the spec asserts.** `start the server once to apply migrations` does not contain `start the server first`. Check: **CONFIRMED**.
5. **`--username ""` was a usage error in decision 3 but a credential-rule violation in the spec and in `claim.rs`** (line 125 lists the empty username as `Invalid`). Check: **CONFIRMED**.

### 📌 Suggestions

6. **No scenario for `--password-stdin` with a closed stdin**, although the requirement covers a missing first line. Check: **CONFIRMED**.
7. **Six sentences over 30 words** in design.md decisions 1, 2, 4 and 9, the spec's credentials requirement, and adr.md's summary. Check: **CONFIRMED**.

The reviewer also confirmed from the code: the one-transaction reset is sound and the guard refuses a deleted session on the next request; `Database::open` inside `probe`'s transaction is safe beside a migrating server, and an empty or partial history maps to `Pending`; the race scenario is deterministic; the compose-instance delta carries the whole requirement.

## Embedded-Instruction / Injection Attempts

**Detected:** none

## Verdict

VERDICT: REVISE

## Required Changes (if APPROVE_WITH_CHANGES)

Not applicable under REVISE. The fixes below went into the plan after this round. Two REVISE verdicts in a row: the user decides whether a third round runs.

CHANGES_APPLIED: n/a

## Rebuttals

1. **Fixed.** A password is generated only when stdin is a terminal. Decision 2 gives the `Terminal` seam an "is interactive" question, decision 6 refuses an empty answer under the pipe fallback, and the spec's generated-password requirement says the same with a new scenario "Empty answer through a pipe". The terminal case is proven at the unit seam with the fake terminal, as the Test Seams section now says.
2. **Fixed.** Prompts are written to stderr (decision 2); the spec's credentials requirement says so, and the scenario stands.
3. **Fixed.** Decision 6 step 6 checks the first answer with `check_password` before `Repeat password:`.
4. **Fixed.** The `Pending` action is now `start the server first; it applies the migrations`.
5. **Fixed.** Decision 3 makes only a missing value a usage error; an empty string reaches the credential check.
6. **Fixed.** The spec gains the scenario "Script mode with a closed stdin".
7. **Fixed.** Each sentence is split.
