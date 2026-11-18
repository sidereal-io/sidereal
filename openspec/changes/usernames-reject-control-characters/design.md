## Context

`check_username` in `server/crates/core/src/access/claim.rs` is the only place the username rules live. It returns a `CredentialRule`, whose `Display` text is what the recovery command prints. Two callers use it:

- `Database::claim`, which turns any broken rule into `ClaimError::Invalid`. The claim handler answers that with 400 `invalid credentials`.
- The create state of `siderealctl` in `server/crates/server/src/siderealctl/prompts.rs`. It maps a broken rule to a refusal of one stderr line and exit 1. This covers both `--username` and the interactive `Username:` prompt.

Today the checks run in a fixed order: length first, then whitespace at the ends. The core unit test `credential_rule_names_the_rule_a_username_breaks` holds that order. One of its cases is `sam\t`, which is both whitespace at the end and a control character.

PostgreSQL's `text` type cannot store a NUL character. Today a claim whose username holds `\u0000` fails at the insert, and `refused` turns that database error into `Unavailable`, so the route answers 503.

## Goals / Non-Goals

**Goals:**

- One new rule in `check_username`. Both callers get it with no change of their own.
- Keep the refusal texts of the existing rules, and the rule each existing case reports.

**Non-Goals:**

- Escaping names when `siderealctl` prints them.
- Any check on sign-in, or on the reset state's `--username` comparison.
- Any change to the web shell.

## Test Seams

- **Primary test seam:** the server's integration suites, which already drive both callers. `claim_with_bad_credentials_is_a_bad_request` in `server/crates/server/tests/access.rs` gains a control-character case. `server/crates/server/tests/siderealctl.rs` gains a spawned test next to `spawned_a_padded_username_creates_nothing` that asserts the exact stderr line.
- **Existing seams reused:** the core unit table `credential_rule_names_the_rule_a_username_breaks` in `claim.rs`. It gains rows for the new rule, and keeps `sam\t` as a whitespace case to fix the check order.

## Scrutiny Tier & Blast Radius

- **Tier:** low
- **Blast radius:** `check_username` and `CredentialRule` in `core`'s access module, and the tests of its two callers. No schema, API shape, dependency or `plugin-abi` change.

## Decisions

**D-1. A new variant, `CredentialRule::UsernameControl`, displayed as `username must not contain control characters`.** The text follows the form of the two existing username rules (brainstorm D8). The alternative was to widen `UsernameWhitespace` to cover control characters. That would merge two rules with different fixes under one message, and the spec names each rule on its own.

**D-2. The check is `username.chars().any(char::is_control)`.** `char::is_control` is true for exactly the Unicode general category Cc: C0, DEL and C1 (brainstorm D1). That is the category the spec names. The security event writer already uses the same test to decide what to escape, at `server/crates/core/src/access/security_event.rs:182`. The alternative was a list of named characters, such as ESC and line feed. That list would miss C1 controls such as U+009B, which some terminals read as the start of an escape sequence.

**D-3. The control check runs last, after length and whitespace.** A username such as `sam\t` breaks both the whitespace rule and the control rule. Running the new check last keeps every existing case on the rule it reports today, so the existing tests and the `Padded username` scenarios stay as they are. A name with a control character inside it, such as `sa\nm`, reaches the new check and reports it.

**D-4. The doc comments follow the rename.** `ClaimError::Invalid` says "break the length rules" and `check_username` describes only length and whitespace. Both change to match the requirement's new name, "the credential rules" (brainstorm D9).

No decision here is costly to reverse, so none needs an ADR.

## Risks / Trade-offs

- [A claim whose username holds `\u0000` changes from 503 to 400.] → This is the right answer for a bad request, not a fault. NUL is in Cc, so the new rule refuses it before the insert. The core unit table gains a NUL row, and no caller depends on the 503.
- [Format characters, such as bidi overrides and zero-width characters, still pass.] → Out of scope by brainstorm D2. They cannot move the cursor or end a line, so they do not break the one-line stdout and stderr contract. A bidi override such as U+202E can still reverse how the rest of a printed line looks. That spoof stays possible until a later change sets a Unicode profile for usernames.

## Migration & Rollback

N/A. No stored administrator can hold a control character, because `main` has never released (brainstorm D6). Rollback is a revert.
