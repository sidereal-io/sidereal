## Why

Sam cannot tell today whether someone else claimed the instance, reset its password from the box, or tried to guess a password. ADR-007 says trust-relevant events are audited, and every source of these events has now shipped: the claim and sessions, `siderealctl`, and the sign-in limit.

## What Changes

- Every security event writes one `security event:` line to the server's normal log, as `key=value` fields with every caller-supplied value escaped (D14). The events are:
  - a claim from the browser or from `siderealctl`;
  - a sign-in that succeeds, fails on wrong credentials, or makes an address limited (D2);
  - a sign-out that ends a session (D6);
  - an access recovery from `siderealctl`, with the number of sessions it ended (D13).
- Nothing is stored in PostgreSQL: no table and no migration (D1).
- Every line names its source, `http` or `siderealctl` (D7). A failed sign-in names no username, only the client address and the user agent (D9). The user agent is cut to 256 characters (D17).
- An event from `siderealctl` reaches the same log as one from the browser, `docker compose logs app`, and also prints on the command's own stderr (D14).
- Refused requests are not security events and write no line (D10, D11, D12). They include a malformed sign-in body (400), a 429 after the first of a run, and a refused claim (409). They also include a `siderealctl` create that loses the race to a browser claim.
- **BREAKING** for anyone who watches the log: the `security event:` line replaces the `sign-in limited:` line, so each event writes exactly one line (D3).
- Out of scope: showing events in the app, log rotation beyond Docker's own, events from later epics, and a logging framework (D15).

## Capabilities

### New Capabilities

- `security-events`: which acts are security events, the `security event:` line and its fields and escaping, how an event from `siderealctl` reaches the server's log, and which refused requests write nothing.

### Modified Capabilities

- `instance-access`: "The server warns when an address becomes limited" asks for the `security event:` line in place of `sign-in limited:`.

## Impact

- **Core:** `server/crates/core/src/access/` gains the event value and its line, so the server and `siderealctl` share one format. `claim` and `reset_admin_password` return the event. A `siderealctl` act sends its line with `pg_notify` in its own transaction, and `core` offers a listener for those lines.
- **Server:** the claim, sign-in and sign-out handlers and `limited()` in `server/crates/server/src/access/mod.rs` pass the client address and user agent and write the line. The server listens for `siderealctl` lines and writes each to its log. `siderealctl` in `server/crates/server/src/siderealctl/mod.rs` also prints its line on its own stderr.
- **Tests:** `server/crates/server/tests/access.rs` and `siderealctl.rs` gain per-path line checks. The existing `sign-in limited:` tests change to the new line.
- **Docs:** `docs/install.md` names the `security event:` line in the sign-in limits section in place of `sign-in limited:`, and says in Recover access that a reset appears in the app's log.
- **Operators:** a log filter on `sign-in limited:` must change to `security event:`. No API, cookie, schema or web change.
