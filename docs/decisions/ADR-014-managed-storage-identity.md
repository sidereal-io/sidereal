---
id: adrs-adr014
date: 2026-10-16
status: accepted
title: 'ADR014: Managed Storage Identity'
description: Identify managed storage through a versioned marker paired with PostgreSQL during startup.
---

# ADR-014: Managed Storage Identity

## Context

Sidereal keeps managed image files on disk and records their metadata in PostgreSQL.
A Docker bind mount names a location, but that location can expose different drives over time.
An unavailable drive can leave an empty directory at the same path.
Treating that directory as new storage would hide the loss and permit writes to the wrong disk.

## Decision

Pair each database with one managed store through an opaque UUID recorded in PostgreSQL and a versioned storage marker.
Initialize during startup only when the database has no pairing and storage is empty or contains only a valid marker.
Once paired, refuse missing or mismatched markers rather than creating replacement storage.

The marker identifies the store's contents independently of its path or physical drive.
Its format contains a version and UUID; readers ignore unknown fields but reject unsupported versions and invalid required fields.
The marker carries no commands, paths, or secrets.

The operator accepted this revised startup initialization decision on 2026-10-19.
Startup adds no HTTP mutation endpoint and retains the network restrictions described in [ADR-007](ADR-007-security-and-plugin-trust.md).
This decision does not waive authentication prerequisites for library operations or plugins.

## Consequences

- Operators retain the one-command installation and do not run a separate storage initialization command.
- A paired database detects a missing marker even when the configured path still exists.
- A marker-only directory can recover an interrupted first installation without manual initialization.
- Operators can move storage intact while retaining the marker and corresponding database.
- Startup cannot distinguish simultaneous database and storage loss from a new installation when both appear empty.
- An unpaired database can adopt any valid marker-only directory; the marker does not prove its installation history.
- A copied marker cannot prove image integrity or prevent another database from using the same store.
- Operators must stop containers before changing mounts; this marker does not monitor live mount changes.

## Alternatives Considered

### Alternative 1: Require a separate initialization command

- **Pros**: Makes permission to create storage explicit, even when both persistence locations are missing.
- **Cons**: Breaks the one-command installation and adds another operation for operators to understand.
- **Why not**: The proposed design uses retained database pairing to distinguish an established store from first initialization.

### Alternative 2: Identify storage by path or physical drive

- **Pros**: Avoids a persistent marker format.
- **Cons**: Paths can expose different contents, and drive identity can change during a valid storage move.
- **Why not**: The application needs to recognize the store independently of its host location.

### Alternative 3: Keep the expected identity only in configuration

- **Pros**: Avoids database pairing metadata.
- **Cons**: Configuration can accept storage independently of the database whose records describe it.
- **Why not**: Database pairing detects an accidental storage swap after initialization.
