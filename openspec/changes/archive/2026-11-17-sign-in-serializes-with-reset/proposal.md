## Why

A sign-in that checked the old password just before `siderealctl reset-admin-password` replaced it can still save its session after the reset has deleted every session (D1). The session survives the reset. Someone with Sam's stolen password can win this race with a script, which defeats the reason Sam ran the reset.

## What Changes

- **Sign-in saves its session only against the hash it checked.** The step that saves the session waits for any reset that holds the administrator's row, and saves nothing when the hash has changed since the check (D6). Such a sign-in answers 401, like a wrong password, and creates nothing (D2).
- **A sign-in that finishes first is unchanged.** It answers 200, and the reset then ends its session, as the reset already does (D2, D7).
- **The issue's method changes.** The issue asked for a transaction held with the row locked across the password check. The lock covers only the save, so a slow password check never holds a database connection (D6). The brainstorm records this under "Changes from the story".
- **A test proves the order.** It holds a reset open, signs in with the old password, then lets the reset commit. The sign-in answers 401 and the `sessions` table stays empty (D4).

Rechecking the hash without a lock, the claim race, and rate limiting stay out (D5).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `instance-access`: the requirement "Sign-in creates a session" says that a sign-in checked against a hash that a password reset has since replaced creates no session and answers 401 (D3). "The reset state sets a new password and ends every session" stays as it is.

## Impact

- **Code:** `Database::sign_in` in `server/crates/core/src/access/session.rs`. The reset in `server/crates/core/src/access/recovery.rs` does not change.
- **Tests:** a new concurrency test in `core` that drives `reset_admin_password` and `sign_in` together.
- **API, cookies, schema, web interface:** no change. A sign-in that loses the race gets the same 401 as a wrong password, so it also counts against the sign-in limit and writes the same wrong-credentials security event line.
