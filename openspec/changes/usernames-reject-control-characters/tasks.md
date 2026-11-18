## 1. Usernames refuse control characters

- [ ] 1.1 [easy] `POST /api/claim` with the username `sa\u{1b}[2Jm` and a valid password answers 400 with `{"error":"invalid credentials"}` and writes no user. Add the case to `claim_with_bad_credentials_is_a_bad_request` first and watch it fail. Then add `CredentialRule::UsernameControl`, displayed as `username must not contain control characters`, and check `username.chars().any(char::is_control)` in `check_username` after the length and whitespace checks, as design D-1 to D-3 say. Update the doc comments on `ClaimError::Invalid` and `check_username` to "the credential rules", as D-4 says (verify: cd server && cargo test -p sidereal-server --test access claim_with_bad_credentials_is_a_bad_request)
- [ ] 1.2 [easy] `check_username` reports `UsernameControl` for a NUL, a line feed inside the name, a delete (U+007F) and a C1 control (U+009B). It still reports `UsernameWhitespace` for `sam\t`, which breaks both rules. Add these rows to the `credential_rule_names_the_rule_a_username_breaks` table (verify: cd server && cargo test -p sidereal-core --lib access::claim::tests::credential_rule_names_the_rule_a_username_breaks)
- [ ] 1.3 [easy] In the create state, `siderealctl reset-admin-password --username` with `sa`, a line feed and `m`, and a valid password on stdin, exits 1. Its stderr is exactly `username must not contain control characters\n`, and the `users` table stays empty. Add the test in the `spawned` module next to `spawned_a_padded_username_creates_nothing` (verify: cd server && cargo test -p sidereal-server --test siderealctl spawned_a_control_character_in_the_username_creates_nothing)
- [ ] 1.4 [easy] The existing claim, sign-in and recovery tests pass unchanged, including both "Padded username" cases (verify: cd server && cargo test -p sidereal-core --lib access && cargo test -p sidereal-server --test access --test siderealctl)

## 2. Integration [depends-on: 1]

- [ ] 2.1 [easy] The whole gate passes (verify: just check)

## Deviations Ledger

<!-- Dated notes on small departures from the plan, written during apply. Stop and ask the user about a large one. -->
