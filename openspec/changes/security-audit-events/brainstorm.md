## Story

#453 security-audit-events — claim, sign-in, sign-out and recovery leave an audit row.
https://github.com/sidereal-io/sidereal/issues/453

## Decisions

| # | Question | Answer | Source |
|---|---|---|---|
| D1 | Which security events leave a row in PostgreSQL? | A claim (browser or `siderealctl`), a `siderealctl reset-admin-password` reset, a sign-in that fails on wrong credentials, and an address becoming limited. A successful sign-in and a sign-out leave no row, only the `security event:` log line. | User |
| D2 | Is an address becoming limited its own kind? | No. It is a failed sign-in with outcome `limited`. | User |
| D3 | Does the `security event:` line replace the `sign-in limited:` line? | Yes. It replaces it and changes the requirement "The server warns when an address becomes limited", so each event writes exactly one line. | User |
| D4 | Can a claim or a reset succeed when its row cannot be written? | No. The row is written in the act's own transaction, and without it the act fails as unavailable: 503 from the browser, exit 1 from `siderealctl`. | User |
| D5 | Does a failed row write change a 401 or 429 sign-in answer? | No. Those answers are unchanged, and they always write the log line. | User |
| D6 | Does a sign-out that ends no session write anything? | No. Only a sign-out answered 204 writes the line; one answered 401 writes no line and no row. | User |
| D7 | How does a row say where it came from? | Every row records a source, `http` or `siderealctl`. A `siderealctl` row has no client address and no user agent. | User |
| D8 | Where do the requirements live? | In a new capability, `security-events`. | User |
| D9 | Does a failed sign-in name a username? | No: neither the typed nor a matched username, only the client address and user agent. | Evidence: #453 Scope |
| D10 | Which refused requests leave no row? | A malformed sign-in body (400), a 429 after the first, and a refused claim (409). | Evidence: #453 Scope |
| D11 | Do those refused requests write a `security event:` line? | No. They are not security events, so they write no line either. | Assumption |
| D12 | Does a `siderealctl` create that loses the race to a browser claim (exit 1) leave a row? | No row and no line, like a refused claim. The browser claim that won leaves its own. | Assumption |
| D13 | What does a reset row add? | The number of sessions the reset ended. | Evidence: #453 Scope |
| D14 | What is the log line? | One stderr line, `security event:` then `key=value` fields, with every caller-supplied value escaped so it cannot forge a line. `siderealctl` writes its line to its own stderr. | Evidence: #453 Scope |
| D15 | What stays out? | A screen for the events, retention or pruning, events from later epics, and a logging framework. | Evidence: #453 Scope |
| D16 | What does the glossary entry for Security event say? | One trust-relevant act on access, written as one log line: an instance claim, a sign-in that succeeds or fails, a sign-out, or an access recovery. A claim, an access recovery and a failed sign-in are also kept as a row in PostgreSQL. | User |

## Changes from the story

- **Cut: rows for a successful sign-in and a sign-out** (D1). Both keep the log line. The user compared Home Assistant, Frigate and Immich; none keeps a durable sign-in history. The issue's Scope and title still list these rows; the pull request description records the narrowing.
- **Changed: the acceptance tests** (D1, D6). Each path that keeps a row writes exactly one row and one line. A successful sign-in and a sign-out answered 204 write exactly one line and no row.
- **Added: a `source` column** (D7), `http` or `siderealctl`.
- **Changed: an existing requirement** (D3). "The server warns when an address becomes limited" in `instance-access` now asks for the `security event:` line in place of `sign-in limited:`.
- **Changed: the glossary entry for Security event** (D16). Only a claim, an access recovery and a failed sign-in are kept as a row; every event writes the log line.

Confirmed by user: yes
