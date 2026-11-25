<!-- Plain language: Lead with the problem and resulting behavior. Give a reader enough context to decide whether the scope solves that problem. Keep the required headings. -->

## Why

Nova uploads a PNG or JPEG and its asset page shows only the file size. Nova
also needs to see the image's format and its width and height in pixels, which
are the next facts the epic's confirmed scope names (#356).

The format is already known: the upload Source sets each version's media
type, such as `image/png`, from the file's bytes. It is not a facet, and it
should not become one owned by `packs/image`. Under ADR-008 one pack owns a
facet's schema, so a closed list of formats in `packs/image` could never take
FITS or XISF from `packs/astro` (D3).

## What Changes

- **New: two facets in `packs/image`** (D1, D2, D5): `image.width` and
  `image.height`, counts in `pixels`, labelled `Width` and `Height`.
- **New: a header Operator in `packs/image`** (D6), separate from the
  file-size Operator. It finds the format from the image's leading bytes, not
  its name, and reads the stored width and height from its header, ignoring
  any EXIF rotation (D8). When a file has a valid signature but its header is
  missing or unreadable, the run fails with a message such as `the PNG header
  could not be read`, and the file size is still recorded (D7).
- **Changed: upload runs both Operators**, the file-size Operator and then the
  header Operator, over each new version. A failed run of either does not fail
  the upload. The server log line for a failed or unrecorded run names the
  Operator's id instead of always saying `file-size`.
- **Changed: reading one Asset** (`GET /api/assets/{id}`) adds `media_type`,
  the current version's media type, to its `asset` object (D11). The library
  list and the upload response keep their shape.
- **Changed: the asset page** shows a `Format` line first for every image,
  from the media type: `image/png` reads `PNG`, `image/jpeg` reads `JPEG`, and
  a media type the web does not know reads as it is (D4). The server's facets
  follow in its order, by facet name, so the page reads `Format PNG`,
  `File size`, `Height 2,160 pixels` and `Width 4,096 pixels` (D10). A failed
  header run shows no error on the page: #527 shows processing failures (D7).
- **Removed: the line `No facts are recorded for this image.`** Every version
  has a media type, so every page has a `Format` line (D12).
- Images uploaded before this change get no dimensions, but do show their
  format (D9).
- `plugin-abi` does not change, and no migration is added.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `asset-facets`: adds the image pack's width and height facets and their
  header Operator; a failed run whose outcome cannot be recorded stays
  `running`.
- `asset-library`: reading one Asset returns its current version's media
  type; a new upload's facets include height and width.
- `asset-page`: shows a `Format` line from the media type before the facts,
  and leaves out an empty unit; drops the no-facts line; shows the facts a
  new upload has, including when only its header run failed.
- `image-upload`: upload runs the header Operator after the file-size
  Operator; a failed or unrecorded run of either is logged with the
  Operator's id and does not fail the upload.

## Impact

- `server/crates/packs/image`: two facet schemas and the header Operator,
  which reads PNG headers with the pure-Rust `png` crate and JPEG headers
  with `zune-jpeg`, the decoders that the `image` crate uses. These are the
  pack's first crates.io dependencies.
- `server/crates/server`: the upload's Operator list, its log lines, and
  `media_type` in the read of one Asset.
- `web/src/AssetPage.tsx`: the `Format` line, an empty unit, and no
  no-facts line.
- No change to `plugin-abi`, core, the database schema, the upload
  response, the Library screen or `DESIGN.md` tokens.
