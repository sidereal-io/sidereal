---
id: adrs-adr020
date: 2026-11-25
status: accepted
title: 'ADR-020: Bound Preview Decoding in a Child Process'
description: Architecture decision record for enforcing a per-preview memory ceiling without sharing decoder allocation failures with the server.
---

# ADR-020: Bound Preview Decoding in a Child Process

## Context

Image previews decode files supplied by users. The memory ceiling covers
output pixels, metadata and decoder scratch space together. The PNG, JPEG
and TIFF libraries do not expose one limit across those allocations. TIFF
can allocate while constructing its decoder, before caller limits take
effect. An allocator failure in the server process can interrupt unrelated
uploads and reads.

[ADR-001](ADR-001-plugin-boundary.md) gives built-in Operators mediated
access to input and output through their Asset context.

## Decision

Run each preview decode and encode in a short-lived first-party helper.
The server ships and supervises it as part of the normal installation. The
helper receives one seekable copy of the candidate and returns JPEG bytes
and an exit status. The host stages the bytes through the Asset context
and publishes them only after success.

The helper enforces its memory budget from the start of decoding. It
charges live allocations, including scratch space and overlapping
allocations during growth, before granting them. It reports budget
exhaustion with a dedicated exit status. The host treats any other
unexpected exit as a failed run and discards the staged output.

## Consequences

- A decoder's budget failure cannot abort the server. Codec updates still
  need format and failure tests, and any new allocation path must use the
  helper's budget.
- The helper adds a process start and a seekable input copy to a preview.
  Its memory enforcement is local to one preview. The host still controls
  how many previews run at once.
- The helper is part of the trusted built-in path. It receives no store
  path or database credentials. Core still grants access and owns all
  persistent writes.
- The packaging and tests must include the helper wherever the server
  runs. The host must reap it on cancellation and check its exit status
  before accepting any bytes it returned.

## Alternatives Considered

- **Estimate every allocation in the server.** This avoids a subprocess,
  but each dependency update can change the constructor and decode paths
  the estimate needs to cover. An omitted allocation can still exhaust
  the server's memory.
- **Use a separate service for image processing.** A service offers a
  stronger runtime boundary. It would add a deployment dependency and a
  connection lifecycle for a built-in operation that has one input and
  can finish in a single local invocation.
