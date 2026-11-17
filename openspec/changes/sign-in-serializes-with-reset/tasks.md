## 1. Sign-in waits for an open reset

- [ ] 1.1 [medium] In the `session.rs` test module, a helper holds `reset_admin_password` open between its hash update and its session delete, as design D-2 says: an advisory lock held by a test connection, and a `BEFORE DELETE ON public.sessions` statement trigger that waits on it. A test proves the helper works: while held, another connection still reads the old hash and the reset task has not finished; after release, the reset returns `Ok` and the new hash is stored. Every wait has a 10-second limit (verify: cd server && cargo test -p sidereal-core --lib access::session::tests::a_held_reset)
- [ ] 1.2 [complex] A sign-in as `sam` with the old password, started while the reset is held, answers `Ok(None)` once the reset commits, and the `sessions` table is empty. Write the test first and watch it fail on `Ok(None)`. Then make `Database::sign_in` save its session with the one statement in design D-1: a `FOR SHARE` lock on the user row, inserting only while `password_hash` still equals the hash it checked, and answering `None` when no row is inserted. Its doc comment says why (verify: cd server && cargo test -p sidereal-core --lib access::session::tests::a_sign_in_during_a_reset)
- [ ] 1.3 [easy] The existing sign-in, session and reset tests pass unchanged: a correct password still answers a user and saves one row with the token's SHA-256, a wrong password and an unknown username save nothing, and a reset still ends every session (verify: cd server && cargo test -p sidereal-core --lib access)
- [ ] 1.4 [easy] The `POST /api/session` and `siderealctl` reset tests pass unchanged, including "Reset while signed in" (verify: cd server && cargo test -p sidereal-server --test access --test siderealctl)

## 2. Integration [depends-on: 1]

- [ ] 2.1 [easy] The whole gate passes (verify: just check)

## Deviations Ledger

<!-- Dated notes on small departures from the plan, written during apply. Stop and ask the user about a large one. -->
