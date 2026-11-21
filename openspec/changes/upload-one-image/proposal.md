## Why

Nova can sign in but cannot put an image in Sidereal. This first upload path
lets Nova see one image in the library and gives Sam durable original bytes;
later import and processing stories need this foundation.

## What Changes

- Introduce Assets and immutable Asset versions with opaque identities,
  current-version references, BLAKE3 hashes and provenance (D5).
- Extend managed storage with staged ingest, durable object publication and
  shared objects for matching hashes, while preserving storage pairing (D6, D9).
- Make the public Source contract executable and register a manual-upload
  Source through a new first-party image pack (D1, D7). **BREAKING:** implementers
  of the current pre-freeze Source stub must implement its execution contract.
- Accept one PNG or JPEG identified from its bytes, up to 100 MiB, with clear
  refusal messages; full decoding is not an upload requirement (D2, D3).
- Add guarded upload, library listing and original-byte endpoints. Show an
  upload button, image names and original images in the signed-in view,
  including an explicit display error for an image the browser cannot render
  (D3, D4, D7).
- Keep Asset deduplication, startup temporary-file cleanup, metadata processing,
  TIFF, previews and richer viewing in their existing follow-up stories (D8).

## Capabilities

### New Capabilities

- `asset-library`: Durable Asset identity, immutable versions, a current-version
  reference, listing and original-byte retrieval.
- `image-upload`: One-image upload through a registered Source, format and size
  refusals, and the signed-in upload and library experience.

### Modified Capabilities

- `managed-storage`: Add safe staged object publication and reuse, commit-last
  durability, and containment of ingest and original-byte reads. Existing
  pairing and marker-only startup requirements remain intact.

## Impact

- `server/crates/plugin-abi`: executable Source and limited run-scoped Asset
  context; the public contract remains intentionally unfrozen.
- `server/crates/core`: asset migration and schema snapshot, persistence,
  BLAKE3 hashing, managed-object operations and Source execution.
- `server/crates/packs/image`: new compiled-in pack depending on `plugin-abi`
  only among project crates; PNG/JPEG recognition lives here.
- `server/crates/server`: register the image pack and wire `/api/assets` and
  original-byte access into the existing session and origin guards.
- Rust workspace manifests, lockfile and architecture lint: add the pack and
  hash dependency; enforce pack isolation for both packs.
- `web/src`: add the library within the existing signed-in view. Preserve
  health checks, access forms, sign-out and event-socket behavior.
- Tests cover storage faults, protected APIs, browser display failures and
  persistence across a server restart. No deployment setting is added.
