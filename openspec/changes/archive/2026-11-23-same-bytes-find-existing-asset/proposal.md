<!-- Plain language: Lead with the problem and resulting behavior. Give a reader enough context to decide whether the scope solves that problem. Keep the required headings. -->

## Why

Today, uploading bytes the library already holds makes a second Asset. The
two Assets share one stored file but show twice in the library. Epic #356's
Shared decision 2 says the same bytes must find the existing Asset instead.
#521 made the first upload work, so this is the next step: without it, a
lost answer or a second try fills the library with copies.

## What Changes

- An upload whose BLAKE3 hash matches a version of any existing Asset finds
  that Asset and commits no new Asset or version (D1). When several Assets
  share the bytes, the upload finds the oldest one (D2). The existing Asset
  keeps its name; the new filename is not stored (D3).
- `POST /api/assets` answers a repeat upload with **409 Conflict** and an
  `asset` object for the existing Asset, with the same fields as the 201
  answer (D4, D9). A first upload still answers 201. This changes the API
  for a repeat upload, which answered 201 with a new Asset before; no client
  outside this repository uses the API yet.
- Two uploads of the same bytes at the same moment make one Asset. The
  other gets the repeat-upload answer (D7).
- The library screen shows a notice that is not a refusal when an upload
  answers 409: "`<chosen name>` is already in the library.", plus
  " as `<existing name>`" when the names differ. The list stays as it is,
  and Nova can choose another file at once (D10).
- The lost-answer message changes. It now says that uploading the file
  again is safe, because Sidereal finds it if it was saved. The screen
  still never retries by itself (D6).
- Not in this change: opening the existing image (no viewer until #528,
  D5), finding near-duplicates, and merging Assets that already share bytes
  (D8).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `asset-library`: adds the rule that an upload of bytes already in the
  library finds the existing Asset, including which Asset, its name, and
  concurrent uploads.
- `image-upload`: adds the 409 answer for a repeat upload, the library
  screen's already-in-the-library notice, and the lost-answer message.
- `managed-storage`: "Matching content shares one stored object" stops
  deferring the library side of a repeat upload to a later change and
  points to `asset-library` instead.

## Impact

- **Core** (`server/crates/core`): `ingest` reports whether it created an
  Asset or found one; the Asset transaction looks for a version with the
  same hash, under a lock that serializes uploads of the same bytes. No
  migration: the content-hash index from #521 serves the lookup.
- **Server** (`server/crates/server/src/assets.rs`): the upload route
  answers 409 with the found Asset.
- **Web** (`web/src/Library.tsx`): handles the 409 answer and changes the
  lost-answer message.
- **Tests**: core ingest tests, the server's in-process Asset route tests,
  the Library component tests, and the upload journey in `web/e2e`.
- **Base**: stacked on #521's branch `upload-one-image` (pull request #532).
