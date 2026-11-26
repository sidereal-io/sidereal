---
id: adrs-adr019
date: 2026-11-25
status: accepted
title: 'ADR-019: Operators Declare the Media Types They Accept'
description: Architecture decision record (ADR) for which Asset versions an Operator runs on. Each Operator declares, at registration, the media types of originals it accepts; its Asset context gives it the version's media type; and core runs an Operator only on an original's version whose media type it accepts. This is the first, media-type-only step towards the accepts Selector.
---

# ADR-019: Operators Declare the Media Types They Accept

## Context

A Source brings a file into the library and records its media type, such as `image/png` or `image/tiff`, from the file's bytes. An Operator then reads that version and proposes facts about it, or makes a new file from it, such as a preview.

Until now an Operator learned nothing about its input except its bytes. Each Operator that cared about the format read the leading bytes again and failed on a format it did not handle. That has three costs. Every Operator repeats work the Source already did. A run over a file the Operator was never meant for is recorded as a failure, so a FITS Operator would record a failed run on every PNG. And nothing stops an Operator from running on its own output: the image pack's Operators handle JPEG, and a preview is a JPEG, so a preview could get a header run and a preview of its own.

[ADR-006](ADR-006-rule-engine-deferral.md) says each Operator declares an `accepts` Selector, and that core dispatches an Operator only when `accepts` matches its subject. The Selector engine, with facets, kinds and Collections, does not exist yet. The upload runs a fixed list of Operators.

## Decision

Each Operator declares the media types it accepts, as a list such as `image/png`, `image/jpeg` and `image/tiff`. The Registry records that list once, at registration, with the Operator's id, version and grants, and refuses an Operator whose list is empty or holds a media type that is not well formed. The list applies to the versions of originals: the Assets a Source brought in.

The Asset context gives an Operator the media type core recorded for the version it reads. An Operator uses it to choose how to read the bytes, and does not read the signature again.

Core runs an Operator only on an original's version whose media type is in the Operator's list. For any other version, core records no operation run and reports that the Operator does not accept it. A derived Asset's version, such as a preview, has a role, and no Operator can yet declare a role it accepts, so no Operator runs on a derived version.

## Consequences

- Recognition happens once, in the Source. An Operator that reads several formats picks its reader from the media type.
- An Operator never records a failed run for a format it does not handle, so failed runs mean a real problem with a file the Operator should read.
- No Operator runs on a preview, by the same rule that keeps a FITS Operator off a PNG. A caller that runs Operators, such as the upload or a later reconciler, needs no rule of its own for derived Assets.
- An Operator that reads any file, such as one that measures file size, must still list each media type. A pack that adds a format adds it to its Operators' lists.
- An Operator trusts the media type core recorded. If the stored bytes do not match it, for example because the object was replaced on disk, the Operator fails reading them as that format; it does not guess another.
- A later Operator that works on derived Assets, such as a thumbnail of a stack, needs a way to name the role it accepts. That extends the declaration; it does not replace it.
- The list is a first step towards the `accepts` Selector. A Selector over facets, kinds and Collections can later take its place, with a media type as one of its terms.

## Alternatives Considered

### Alternative 1: Each Operator reads the signature and fails on other formats
- **Pros**: no change to the contract; each Operator checks the bytes it is about to read.
- **Cons**: every Operator repeats the Source's recognition; every run over an unhandled format is recorded as a failure; nothing keeps an Operator off a preview.
- **Why not**: the failed runs would be noise that hides real failures, and keeping Operators off previews would need a separate rule.

### Alternative 2: The full `accepts` Selector now
- **Pros**: the declaration ADR-006 describes, in one step.
- **Cons**: needs the Selector engine and its index over facets, kinds and Collections, none of which exist.
- **Why not**: only the media type is needed today, and a list of media types can become one term of a Selector later.

### Alternative 3: The caller chooses which Operators run on which version
- **Pros**: no change to the contract; the upload already holds its list of Operators.
- **Cons**: what an Operator can read would live in the server, away from the pack that knows it; each caller would need its own rule to skip previews.
- **Why not**: the pack owns its formats, and a rule kept in one caller is missed by the next one.

### Alternative 4: Accept by kind, such as `image`
- **Pros**: one value per Operator; already recorded on every Asset.
- **Cons**: a PNG, a TIFF and a FITS file are all images, and a preview is one too.
- **Why not**: a kind cannot tell the formats an Operator reads from those it does not, nor an original from its preview.
