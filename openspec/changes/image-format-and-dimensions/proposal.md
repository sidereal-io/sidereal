<!-- Plain language: Lead with the problem and resulting behavior. Give a reader enough context to decide whether the scope solves that problem. Keep the required headings. -->

## Why

Nova uploads a PNG or JPEG and its asset page shows only the file size. Nova
also needs to see the image's format and its width and height in pixels, which
are the next facts the epic's confirmed scope names (#356). The facet contract
that #524 built has only one value type, a count, so the format cannot be
recorded yet.

## What Changes

- **New: a choice value type in the public plugin contract** (D3). A facet
  schema can declare a choice with its allowed values, and an Operator proposes
  one of them as text. Core refuses any other text, keeps the allowed values
  with the stored schema version, and refuses a run whose pack changed them
  without a new schema version. **BREAKING** for `plugin-abi`: `FacetType` and
  `FacetValue` gain variants that hold text, so neither stays `Copy`, and
  `ABI_VERSION` rises to 3. No third-party pack uses the contract yet.
- **New: three facets in `packs/image`** (D1, D2, D4, D5): `image.format`, a
  choice of `PNG` or `JPEG`, labelled `Format`, with no unit; and
  `image.width` and `image.height`, counts in `pixels`, labelled `Width` and
  `Height`.
- **New: a header Operator in `packs/image`** (D6), separate from the
  file-size Operator. It reads the format and the stored width and height from
  the image's bytes, not its name, and ignores any EXIF rotation (D8). When a
  file has a valid signature but its header is missing or unreadable, the run
  fails with a message such as `the PNG header could not be read`, and the
  file size is still recorded (D7).
- **Changed: upload runs both Operators**, the file-size Operator and then the
  header Operator, over each new version. A failed run of either does not fail
  the upload. The server log line for a failed or unrecorded run names the
  Operator's id instead of always saying `file-size`.
- **Changed: reading an Asset** returns a choice facet's value as a JSON
  string, such as `"PNG"`, and a count's value as a number, as now.
- **Changed: the asset page** shows a choice value as written, and leaves out
  an empty unit, so the page reads `Format PNG`, `Height 2,160 pixels` and
  `Width 4,096 pixels` beside `File size`. Facts stay in the server's order,
  by facet name (D10). A failed header run shows no error on the page: #527
  shows processing failures (D7).
- Images uploaded before this change get no format or dimensions (D9).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `asset-facets`: adds the choice value type, its declaration checks and how a
  kept schema version holds its allowed values; adds the image pack's format,
  width and height facets and their Operator.
- `asset-library`: reading an Asset returns a choice value as a string, and a
  new upload's facets include format, height and width.
- `asset-page`: shows a choice value as written and leaves out an empty unit;
  shows the facts a new upload has, including when only its header run failed.
- `image-upload`: upload runs the header Operator after the file-size
  Operator; a failed or unrecorded run of either is logged with the
  Operator's id and does not fail the upload.
- `postgres-foundation`: a sixth migration, which adds allowed values to the
  kept facet schemas; a first start and a concurrent start hold six
  migration records, and an upgrade from five keeps every stored row.
- `compose-instance`: a first Compose start holds six successful migration
  records.

## Impact

- `server/crates/plugin-abi`: `FacetType`, `FacetValue`, the Registry's
  declaration checks, and `ABI_VERSION`. A public contract change.
- `server/crates/core`: the proposal check, the schema store and its
  comparison, the facet read, and one forward-only migration that adds the
  allowed values to `facet_schemas`, with its schema baseline. The startup
  tests and `scripts/instance-verify.sh` expect six migrations.
- `server/crates/packs/image`: three facet schemas and the header Operator,
  with a PNG and JPEG header reader. No new dependency.
- `server/crates/server`: the upload's Operator list, its log lines and the
  facet JSON.
- `web/src/AssetPage.tsx`: accepts and shows text values, and an empty unit.
- No change to the upload response, the Library screen or `DESIGN.md` tokens.
