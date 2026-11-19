## Story

#456 session-sweep — expired sessions are deleted on a schedule, so the sessions table holds only live sessions. https://github.com/sidereal-io/sidereal/issues/456

## Decisions

| # | Question | Answer | Source |
|---|---|---|---|
| D1 | Which sessions does a sweep delete? | Every row whose expiry has passed (`expires_at <= now()`), and nothing else. Expiry already moves 30 days past the last use, so no separate inactivity rule is needed. | Evidence: `server/crates/core/src/access/session.rs`, `openspec/specs/instance-access/spec.md` |
| D2 | When does the sweep run? | First sweep 1 minute after the server starts, then every hour, fixed in code. | User |
| D3 | What does the sweep log, and is it a security event? | A plain stderr line with the count, such as `session sweep: deleted 3 expired sessions`, only for a sweep that deleted rows. Not a security event. | User |
| D4 | What happens when PostgreSQL cannot be reached during a sweep? | Write one line that starts `database unavailable:`, the prefix the relay already uses, and try again at the next interval. The server keeps serving. | User |
| D5 | What happens to the sweep on graceful shutdown? | It stops with the server; the next start schedules a new first sweep. | Evidence: #456 scope |
| D6 | What is out of scope? | A general background job runner, a configurable schedule, and sweeping anything other than sessions. | Evidence: #456 scope |
| D7 | Which process runs the sweep? | The server only; `siderealctl` does not. | Evidence: #456 scope |

## Changes from the story

None.

Confirmed by user: yes
