## Story

#463 usernames-reject-control-characters — a username cannot carry control characters, so the administrator's name prints as typed. https://github.com/sidereal-io/sidereal/issues/463

## Decisions

| # | Question | Answer | Source |
|---|---|---|---|
| D1 | Which characters does the new rule refuse? | Any character for which `char::is_control` is true (Unicode Cc: C0, DEL and C1), anywhere in the username. | Evidence: #463 packet, Scope |
| D2 | Does the rule cover a wider character policy, such as bidi or zero-width format characters? | No. A Unicode profile for usernames is out of scope. | Evidence: #463 packet, Scope |
| D3 | Where does the rule apply? | In `check_username`, so in `POST /api/claim` (400, `invalid credentials`, writes nothing) and in the recovery command's create state (one stderr line naming the rule, exit 1). | Evidence: `server/crates/core/src/access/claim.rs:64`, `server/crates/server/src/siderealctl/prompts.rs:92` |
| D4 | Do sign-in or the reset state's `--username` check change? | No. Sign-in does not run the credential rules, and the reset state compares `--username` with the stored name without printing the given value. | Evidence: `server/crates/server/src/siderealctl/prompts.rs:76` |
| D5 | Does `siderealctl` escape names on output? | No. It prints names unchanged, because the rule guarantees they hold no control character. Security event lines already escape the username. | Evidence: #463 packet, Scope; `server/crates/core/src/access/security_event.rs:172` |
| D6 | What about an administrator already stored with a control character? | None can exist: `main` has never released, and renaming an administrator is out of scope. | Evidence: AGENTS.md, "`main` does not release yet"; #463 packet, Scope |
| D7 | Does the web shell change? | No. It holds no username rules of its own and shows the server's refusal as it does today. | Evidence: `web/src/Access.tsx` |
| D8 | What text names the new rule? | `username must not contain control characters`, in the same form as the two existing username rules. The recovery command's create state prints it on stderr. | User |
| D9 | Is the requirement "Claimed credentials meet the length rules" renamed, now that a rule not about length joins it? | Yes. It becomes "Claimed credentials meet the credential rules", and the reference to it in "The recovery command applies the claim's credential rules" changes to match. | User |

## Changes from the story

None

Confirmed by user: yes
