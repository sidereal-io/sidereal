# Glossary

Shared vocabulary for Sidereal. A spec folder under `openspec/specs/` may hold its own
`glossary.md` for a narrower meaning. Agree a term with the user before adding it.

## Access

- **Instance claim** — the one-time act that creates an instance's first administrator.
  Open to the first visitor, or done from the box with `sidectl`, and closed forever once
  an administrator exists.
- **User** — an account that can sign in: a username, a password and a role.
- **Role** — what a user may do: `admin` or `viewer`. Only `admin` is in use.
- **Administrator** — the user with the `admin` role. This release has exactly one.
- **Session** — one signed-in browser or client, from sign-in until sign-out, expiry or
  revocation. PostgreSQL holds it; the browser holds only an opaque token in a cookie.
- **Access recovery** — regaining administrator access from the box with `sidectl`, which
  sets a new password and ends every session.
- **Trusted proxy** — a reverse proxy whose forwarded headers the server believes, named
  in configuration.
