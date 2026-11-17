## Story

#462 sign-in-serializes-with-reset — a sign-in in flight during a password reset cannot outlive it. https://github.com/sidereal-io/sidereal/issues/462

## Decisions

| # | Question | Answer | Source |
|---|---|---|---|
| D1 | Is the race real in the code today? | Yes. `sign_in` reads the hash, verifies it, then inserts the session as separate autocommit statements; a sign-in that read the old hash can insert after the reset's `DELETE` runs and keep its session. | Evidence: `server/crates/core/src/access/session.rs`, `server/crates/core/src/access/recovery.rs` |
| D2 | What must hold after the fix? | No session created by a sign-in with the replaced password outlives the reset. A sign-in that waits on an open reset answers 401 and creates nothing. A sign-in that commits before the reset ends keeps a 200, and the reset ends its session. | Evidence: #462 Scope |
| D3 | Which spec carries the rule? | A delta on the requirement "Sign-in creates a session" in `instance-access`. "The reset state sets a new password and ends every session" stays as it is. | Evidence: #462 Scope |
| D4 | What proves it? | A concurrency test: hold a reset open, sign in with the old password, commit the reset; the sign-in answers 401 and the `sessions` table stays empty. | Evidence: #462 Scope |
| D5 | What stays out? | Rechecking the hash without a lock, the claim-versus-create race, and rate limiting. | Evidence: #462 Scope |
| D6 | Where does the row lock go: across the password check (as the packet says), or only around the session insert after the check? | Only around the insert: it locks the user row and saves the session only if the hash still equals the one checked. Same result and test, and a burst of sign-ins cannot hold the pool's 5 connections. | User |
| D7 | Does acceptance also need a test for the opposite order, where the sign-in commits first and the reset then ends its session? | No. That order is an ordinary saved session the reset deletes, which "Reset while signed in" already covers, and a one-statement window makes the test unreliable. | User |

## Changes from the story

- **Method changed (D6).** The issue says to hold a transaction with the user row locked across the password check. The plan locks the row only for the session insert after the check, and the insert requires the hash to still equal the one checked. The outcome and the acceptance test are the same; the change keeps a slow password check from holding a pool connection.

Confirmed by user: yes
