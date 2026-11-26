# Glossary

Shared vocabulary for Sidereal. A spec folder under `openspec/specs/` may hold its own
`glossary.md` for a narrower meaning. Agree a term with the user before adding it.

## Access

- **Instance claim** — the one-time act that creates an instance's first administrator.
  Open to the first visitor, or done from the box with `siderealctl`, and closed forever once
  an administrator exists.
- **User** — an account that can sign in: a username, a password and a role.
- **Role** — what a user may do: `admin` or `viewer`. Only `admin` is in use.
- **Administrator** — the user with the `admin` role. This release has exactly one.
- **Session** — one signed-in browser or client, from sign-in until sign-out, expiry or
  revocation. PostgreSQL holds it; the browser holds only an opaque token in a cookie.
- **Access recovery** — regaining administrator access from the box with `siderealctl`, which
  sets a new password and ends every session.
- **Trusted proxy** — a reverse proxy whose forwarded headers the server believes, named
  in configuration.
- **Client address** — the address a request came from: the connecting peer, or, when that
  peer is a trusted proxy, the nearest address in `X-Forwarded-For` that is not a trusted
  proxy.
- **Allowed origin** — an origin whose state-changing requests the server accepts: the
  request's own host and port, plus any listed in `SIDEREAL_ALLOWED_ORIGINS`.
- **Security event** — one trust-relevant act on access, written as one line in the
  server's log: an instance claim, a sign-in that succeeds or fails, a sign-out, or an
  access recovery.

## Assets and processing

- **Asset** — one logical file in the library, such as a photo. Its identity is stable and
  opaque: renaming or moving the file, or uploading the same bytes again, does not change it.
  Every asset has an `id`, a `kind` and a display `name`. Everything else about it is a facet.
- **Asset version** — one exact set of bytes of an asset, identified by its BLAKE3 hash. A
  version never changes. Changing the bytes makes a new version; a rename or move does not.
- **Facet** — one named, typed fact about an asset or an asset version, such as an image's
  width. One pack owns each facet's schema. Only core writes facet values: it checks each value
  first and records who produced it.
- **Facet schema** — the declaration of one facet: its name, value type, unit, whether it
  describes an asset or an asset version, and the pack that owns it.
- **Grant** — permission for one plugin to write the values of one facet. A pack built into
  the server has its grants from the build.
- **Source** — a plugin that brings files into the library as assets, such as manual upload or
  a watched folder.
- **Operator** — a plugin that takes asset versions and settings, and proposes facets, new
  assets, or both. Metadata extraction is an example.
- **Asset context** — the only way a plugin reaches the library during a run. Through it, the
  plugin can read its input's bytes, propose facets and assets, write log lines and check
  whether it was cancelled.
- **Processing goal** — an outcome that must become true for one asset version, such as
  "metadata extracted". Its state is `pending`, `running`, `blocked`, `satisfied` or
  `needs_attention`.
- **Operation run** — the record of one attempt by an operator: which operator and version ran,
  which goal it served, which versions it read and produced, and how it ended.
- **Lineage** — a recorded link that says which asset versions another version was made from.
- **Derived asset** — an asset an operator made from another asset's version, such as a
  preview. Lineage links it to its source.
- **Preview** — a derived asset holding a browser-ready image made from one asset version at
  full resolution. The library and asset page show it in place of the original.
- **Original** — an asset a Source brought into the library, as opposed to a derived asset.
  The library lists originals only.
