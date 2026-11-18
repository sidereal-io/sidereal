## Why

Today a username may hold any character except whitespace at its ends. On an exposed, unclaimed instance, whoever claims first picks the name. `siderealctl reset-admin-password` prints that name unchanged. A newline or an ANSI escape sequence in it then runs in Sam's terminal, the moment Sam runs the one tool meant to take control back. It can retitle the window, erase lines or fake the state line, and it breaks the command's one-line stdout and stderr contract.

## What Changes

- A new credential rule refuses a username that holds any control character, anywhere in it (D1). It does not set a wider character policy (D2).
- The rule applies wherever the username rules already apply: `POST /api/claim` and the recovery command's create state (D3). Sign-in and the reset state's `--username` check do not change (D4).
- A broken rule is refused with the text `username must not contain control characters` (D8). The claim route answers 400 `invalid credentials` and writes nothing. The recovery command prints that one line on stderr and exits 1.
- `siderealctl` keeps printing names unchanged, because the rule means a stored name holds no control character (D5, D6). The web shell does not change (D7).
- The requirement "Claimed credentials meet the length rules" is renamed "Claimed credentials meet the credential rules" (D9). The recovery command's requirement that cites it changes to match.
- The security-events scenario "A line feed in a claimed username" can no longer happen, because that claim now gets a 400. Its requirement is replaced by one with a new name, "Caller-supplied values are escaped in a line", that keeps the escaping rule and the other two scenarios (D10).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `instance-access`: "Claimed credentials meet the length rules" is renamed "Claimed credentials meet the credential rules" and gains the control-character rule and a scenario. "The recovery command applies the claim's credential rules" cites the new name and gains a scenario for the create state.
- `security-events`: "Caller-supplied values cannot forge a line" is removed and replaced by "Caller-supplied values are escaped in a line". The text is the same; only the scenario "A line feed in a claimed username" goes.

## Impact

- `server/crates/core/src/access/claim.rs`: a new `CredentialRule` variant, its `Display` text, and the check in `check_username`.
- `server/crates/server/tests/access.rs` and `server/crates/server/tests/siderealctl.rs`: new cases for the claim route and the recovery command. The test `claim_escapes_a_line_feed_in_the_username` is deleted.
- `openspec/specs/instance-access/spec.md` and `openspec/specs/security-events/spec.md`: through the deltas, on archive.
- No change to the API's shape, the schema, dependencies or the web shell.
