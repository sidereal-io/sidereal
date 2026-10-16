---
id: adrs-adr014
date: 2026-10-16
status: accepted
title: 'ADR014: Managed Storage Identity'
description: Pair managed storage with PostgreSQL through an explicit initialization command and a persistent identity marker.
---

# ADR-014: Managed Storage Identity

## Context

Sidereal keeps managed image files on disk and records their metadata in PostgreSQL.
A Docker bind mount names a location, but that location can expose different drives over time.
An unavailable drive can leave an empty directory at the same path.
Treating that directory as new storage would hide the loss and permit writes to the wrong disk.

## Decision

Pair each database with one managed store through an opaque identity recorded in both PostgreSQL and a storage marker.
Require an explicit local initialization command to establish that pairing; ordinary startup validates it and refuses absent or mismatched identities.
Bind identity to the store's contents, so moving those contents does not require changing identity.

The operator approved explicit initialization, database pairing, and marker validation during design discussion on 2026-10-16.
The local command requires host or container execution access and exposes no HTTP initialization endpoint.
This respects the network mutation boundary in [ADR-007](ADR-007-security-and-plugin-trust.md).
It does not waive authentication prerequisites for future library operations or plugins.

## Consequences

- Startup distinguishes the paired store from an empty directory or an unrelated store.
- Operators can change mount paths while preserving the marker and corresponding database.
- Initialization adds one deliberate step to first installation.
- Interrupted initialization needs durable progress and safe retry behavior across the filesystem and database.
- Operators must retain the database and storage together; a copied marker cannot prove that every image remains intact.
- The marker provides accidental-misconfiguration detection, not protection against an operator who can rewrite both sides.
- Startup validation cannot guarantee detection of a mount change after the check completes.

## Alternatives Considered

### Alternative 1: Identify storage by path or physical drive

- **Pros**: Avoids a persistent marker.
- **Cons**: Paths can expose different contents, and drive identity can change during a valid storage move.
- **Why not**: The application needs to recognize the store independently of its host location.

### Alternative 2: Initialize automatically when storage appears empty

- **Pros**: Removes a first-install command.
- **Cons**: Missing storage can look exactly like a new installation.
- **Why not**: Startup must never turn unavailable storage into a replacement store.

### Alternative 3: Keep the expected identity only in configuration

- **Pros**: Avoids database pairing metadata.
- **Cons**: Configuration can accept storage independently of the database whose records describe it.
- **Why not**: Recording both sides together makes an accidental database or storage swap detectable.
