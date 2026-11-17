## Story

#453 security-audit-events — claim, sign-in, sign-out and recovery each write one line to the log.
https://github.com/sidereal-io/sidereal/issues/453

## Decisions

| # | Question | Answer | Source |
|---|---|---|---|
| D1 | Where is a security event kept? | Only as one `security event:` line in the server's normal log. Nothing is stored in PostgreSQL: no table, no migration, and no separate log file. Decided 2026-11-15, replacing the earlier rows. | User |
| D2 | Is an address becoming limited its own kind? | No. It is a failed sign-in with outcome `limited`. | User |
| D3 | Does the `security event:` line replace the `sign-in limited:` line? | Yes. It replaces it and changes the requirement "The server warns when an address becomes limited", so each event writes exactly one line. | User |
| D6 | Does a sign-out that ends no session write anything? | No. Only a sign-out answered 204 writes the line; one answered 401 writes no line. | User |
| D7 | How does a line say where it came from? | Every line names a source, `http` or `siderealctl`. A `siderealctl` line has no client address and no user agent. | User |
| D8 | Where do the requirements live? | In a new capability, `security-events`. | User |
| D9 | Does a failed sign-in name a username? | No: neither the typed nor a matched username, only the client address and user agent. | Evidence: #453 Scope |
| D10 | Which refused requests write no line? | A malformed sign-in body (400), a 429 after the first, and a refused claim (409). | Evidence: #453 Scope |
| D11 | Are those refused requests security events? | No. They are not security events, so they write no line. | Assumption |
| D12 | Does a `siderealctl` create that loses the race to a browser claim (exit 1) write a line? | No, like a refused claim. The browser claim that won writes its own. | Assumption |
| D13 | What does a reset line add? | The number of sessions the reset ended. | Evidence: #453 Scope |
| D14 | What is the log line, and where does it go? | One line, `security event:` then `key=value` fields, with every caller-supplied value escaped so it cannot forge a line. Every event reaches the normal log, `docker compose logs app`, including an event from `siderealctl`. | User |
| D15 | What stays out? | Showing events in the app, log rotation beyond Docker's own, events from later epics, and a logging framework. | Evidence: #453 Scope |
| D16 | What does the glossary entry for Security event say? | One trust-relevant act on access, written as one line in the server's log: an instance claim, a sign-in that succeeds or fails, a sign-out, or an access recovery. | User |
| D17 | How long can a user agent be? | Its first 256 characters, with invalid UTF-8 bytes as U+FFFD. | User |

D4 and D5, about a claim, a reset or a failed sign-in whose row cannot be written, were removed with the rows on 2026-11-15.

## Changes from the story

- **Changed: an existing requirement** (D3). "The server warns when an address becomes limited" in `instance-access` now asks for the `security event:` line in place of `sign-in limited:`.

Confirmed by user: yes
