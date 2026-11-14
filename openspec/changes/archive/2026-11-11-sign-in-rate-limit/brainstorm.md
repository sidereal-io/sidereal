## Story

#452 sign-in-rate-limit — failed sign-ins are slowed per client address,
https://github.com/sidereal-io/sidereal/issues/452

## Decisions

| # | Question | Answer | Source |
|---|---|---|---|
| D1 | Which sign-in answers count as a failure? | Only a 401: a wrong password or an unknown username. A 400 (malformed body) and a 503 (database unavailable) do not count. | User |
| D2 | What are the fixed limits? | Per client address, three sliding-window tiers, as Frigate's `1/second;5/minute;20/hour`: at most 1 failure per second, 5 per minute and 20 per hour. An attempt that would pass any tier answers 429 until that tier has room again. | User |
| D3 | How is a limited address answered? | Every `POST /api/session` from it answers 429 before any credential check, even with the correct password, with no password-hash work. These 429s do not count as failures and do not make the wait longer. | User |
| D4 | Does a successful sign-in clear the count? | Yes: it clears every tier's failures for its client address. | User |
| D5 | How are requests with no resolved client address counted? | All of them share one counter, kept apart from every real address. | User |
| D6 | How are addresses grouped? | IPv6 by its /64 prefix; IPv4 by the full address. | User |
| D7 | What does the limited answer look like? | Status 429, body `{"error":"too many attempts"}`, and `Retry-After` in whole seconds, rounded up, at least 1. | User |
| D8 | Does the server log a limit? | One warning line when an address first passes a tier, naming the client address and the wait, never the username. | User |
| D9 | What does the web sign-in screen show for a 429? | A static message, `Too many failed sign-ins. Try again in N …`, with the form still usable. N is in seconds when `Retry-After` is under a minute, and in minutes rounded up otherwise. | User |
| D10 | Where are the fixed limits documented? | A short "Sign-in limits" section in `docs/install.md`, next to the reverse-proxy guide. | User |
| D11 | Where do the counters live? | In the server process. A restart clears them. | Evidence: #452 packet, Scope |
| D12 | Is the story's scope as written in #452, with no split and no new glossary term? | Yes. | User |

## Changes from the story

None.

Confirmed by user: yes
