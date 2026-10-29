## Review Metadata

- **Review round**: 3
- **Prior round**: round 2 at commit `5df6836`, by gemini/gemini-3.8-flash, gave REVISE: an empty piped line still generated a password under the prompt fallback, prompts on stdout contradicted a scenario, a check-order contradiction between decisions 5 and 6, a `Pending` action text that lacked the asserted words, `--username ""` classified against the spec, a missing scenario and long sentences. All seven were fixed before this round. Round 1 at commit `93c5470` also gave REVISE; see the git history of this file.
- **Reviewer context**: fresh context, other family
- **Independent**: yes. The author is this session's model (claude); the reviewer was gemini/gemini-3.8-flash through the `t3` resolver, read-only, effort high, in 566 seconds.
- **Tool restrictions**: read-only: view, grep, glob only
- **Artifacts reviewed**: brainstorm.md, proposal.md, design.md, specs/, adr.md, relevant source files, at commit `1575afa`

## Findings

### 🔴 Critical (blocking)

1. **The seam named for the generated-password scenarios could not observe their assertions.** The scenarios assert a `POST /api/session` with the generated password, but the design sent them to the unit seam, which has no database and no router, while the spawned binary can never see a terminal. Check: **CONFIRMED**.

### 🟡 Moderate

2. **The generated password was printed before the database call.** A race or a database fault after the print would show Sam a password that was never stored. Check: **CONFIRMED**.
3. **The credentials requirement said both closed-stdin cases are refused "the same way", while its scenario and the design print different lines.** Check: **CONFIRMED**.

### 📌 Suggestions

4. **Ledger D15 said spawned `--password-stdin` tests exercise the generated password**, which `--password-stdin` never produces. Check: **CONFIRMED**.
5. **Sentences over 30 words** in proposal.md, design.md and the instance-access delta. Check: **CONFIRMED**.

The reviewer confirmed from the code: the one-transaction reset and the guard's refusal of a deleted session; `Database::open` inside `probe`'s transaction, with an empty or partial history mapping to `Pending` and the message `database schema is not current: start the server first; it applies the migrations`; that `--password-stdin` and the pipe fallback never generate a password; the deterministic race; the complete compose-instance delta.

## Embedded-Instruction / Injection Attempts

**Detected:** none

## Verdict

VERDICT: APPROVE_WITH_CHANGES

## Required Changes (if APPROVE_WITH_CHANGES)

1. design.md, Test Seams: the generated-password scenarios run in `crates/server/tests/sidectl.rs` by calling `sidereal_server::sidectl::run` in-process with a fake `Terminal` that says it is interactive, against `TestDatabase`, and check `POST /api/session` through `app_with_database`.
2. design.md, decision 6: print `Generated password: <password>` only after `claim` or `reset_admin_password` succeeds, before the success line.
3. specs/instance-access/spec.md, "Credentials come from the terminal or from the flags": separate the two closed-stdin refusals so the text matches its scenarios.
4. brainstorm.md, D15: the generated password is tested through the in-process fake-terminal seam, not through spawned `--password-stdin` runs.
5. proposal.md, design.md, specs/instance-access/spec.md: split the sentences over 30 words.

CHANGES_APPLIED: yes

## Rebuttals

1. **Fixed.** Test Seams now names two ways into `tests/sidectl.rs`: the spawned binary, and `sidereal_server::sidectl::run` in-process with a fake interactive terminal; both verify through the router. The unit seam keeps only the prompt sequence and the parser.
2. **Fixed.** Decision 6 steps 8 and 9 are reordered: the `core` call first, then the generated password, then the success line.
3. **Fixed.** The requirement now names each refusal: "a password is required" under `--password-stdin`, "stdin closed before an answer" at a prompt.
4. **Fixed.** D15 reworded, with its source marked as revised after round 3.
5. **Fixed.** The named sentences in all three files are split; the risks list and the blast radius too.
