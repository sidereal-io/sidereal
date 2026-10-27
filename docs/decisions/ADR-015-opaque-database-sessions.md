---
id: adrs-adr015
date: 2026-10-27
status: accepted
title: 'ADR015: Opaque Database-Backed Sessions'
description: Architecture Decision Record (ADR) for browser sessions as opaque random tokens whose hashes live in PostgreSQL, instead of signed tokens.
---

# ADR-015: Opaque Database-Backed Sessions

## Context

[ADR-007](ADR-007-security-and-plugin-trust.md) requires browser access through secure, HTTP-only, same-site sessions that can be revoked, and WebSocket authentication that rechecks expiry and revocation. It does not say what a session is. Two shapes are common: a signed token the server verifies without storage, such as a JWT, or an opaque random token the server looks up on every request. Sidereal is a single-box self-hosted service with one administrator, a PostgreSQL database that every request can already reach, and an operator who must be able to end every session from the box when a password is reset. A signing key would be a second secret to generate, store and rotate on that box.

## Decision

A session is an opaque random token of 32 bytes. The browser holds the token in the session cookie. PostgreSQL holds the SHA-256 hash of the token in a `sessions` row with the user, an authentication type, and created, last-used and expiry times. Every authenticated request looks the row up; a session ends when its row is deleted or expired.

## Consequences

- Revocation is a row delete, so sign-out, password reset and administrator recovery end sessions at once, with no denylist and no waiting for a token to expire.
- No signing key exists, so there is no key to keep, rotate or leak.
- A database read on every authenticated request. The row is small and indexed by hash; the readiness probe already reaches the database on the same path.
- The authentication type column lets a later OAuth sign-in create the same kind of session, so middleware and the cookie do not change when a second sign-in method arrives.
- A stolen database dump exposes only hashes, which cannot be presented as cookies.
- A session cannot be validated while PostgreSQL is unavailable. The server already refuses to be ready without the database, so this adds no new failure mode.

## Alternatives Considered

### Alternative 1: Signed tokens (JWT)
- **Pros**: no storage, no lookup, and the token carries the user and role.
- **Cons**: revocation needs a denylist or short lifetimes with refresh tokens, which brings the storage back; a signing key becomes a second secret on the box; a leaked key forges any session until rotated.
- **Why not**: every request reaches PostgreSQL anyway, and ending every session from the box is a requirement that a stateless token makes indirect.

### Alternative 2: Server-side sessions in process memory
- **Pros**: no table and the fastest lookup.
- **Cons**: every restart signs everyone out; a second process cannot share them; recovery from the box cannot see them.
- **Why not**: a session must survive a restart and be visible to `sidectl`, which runs as a separate process.
