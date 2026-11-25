<!-- Plain language: Lead with the problem and resulting behavior. Give a reader enough context to decide whether the scope solves that problem. Keep the required headings. -->

## Why

Astrophotographers export finished images as TIFF, but upload refuses TIFF,
and browsers could not show one anyway. Nova needs to upload a TIFF and see
it in the library and on its page. A preview (a browser-ready copy of the
image, at full resolution) solves this. Making it is also the first time an
Operator produces a new asset. So this change also sets how a derived asset
and its lineage link are recorded, which later outputs such as stacks and
thumbnails will reuse.

## What Changes

- **Upload accepts TIFF.** The manual-upload Source recognizes classic TIFF
  and BigTIFF, in either byte order, from the file's leading bytes (D4). The
  refusal message and the web interface now name PNG, JPEG and TIFF.
- **The header Operator reads TIFF.** It records `image.format` `TIFF` and
  the width and height of the first image in the file. `image.format` gets
  schema version 2, whose allowed values add `TIFF`. Values already stored
  keep version 1.
- **The plugin contract lets an Operator propose a derived asset.** An
  Operator that declares it makes previews can propose one new asset per
  run and stream its bytes through the Asset context. The plugin ABI version
  goes from 3 to 4. Core still mints every id and computes every hash.
- **Core records derived assets and lineage.** Core stores the Operator's
  output as a new asset and version, marks the asset as derived, and records
  a lineage link from the new version to the version it was made from, with
  the run that made it. A failed or refused run commits no asset.
- **The image pack adds a preview Operator.** It decodes PNG, JPEG and the
  supported TIFF types, and writes an 8-bit JPEG at quality 90 (D3). 16-bit
  and decimal values are scaled straight to 8-bit, with no brightening (D5).
  A TIFF it cannot decode, or an image whose decoded pixels need more than
  512 MiB, fails the run with a reason (D4, D6).
- **Upload makes a preview of every new original** (D1). Upload runs the
  preview Operator after the file-size and header runs, and before it
  answers (D7). A failed preview run does not fail the upload. A preview gets
  no facets and no preview of its own (D9).
- **The library and asset page show the preview.** The API gives each
  Asset a `preview_url`, or `null` when it has no preview, and serves the
  preview's bytes at `/api/assets/{id}/preview`. The web interface shows the
  preview, or the original when there is none (D8). No preview is made for
  Assets uploaded before this change.
- **The library lists originals only.** The list, the Asset read and the
  original route treat a derived asset as not in the library (404). A
  repeat upload is found only among originals, so bytes that match a
  preview make a new original (D10).

## Capabilities

### New Capabilities

- `derived-assets`: how an Operator proposes a derived asset through the
  Asset context, how core checks it, and how core records the asset, its
  version and its lineage link. It also says that an original is an asset
  a Source brought in.
- `image-preview`: the image pack's preview Operator: which images it
  decodes, how it turns their pixels into an 8-bit JPEG, its memory limit,
  and its failure reasons.

### Modified Capabilities

- `image-upload`: upload recognizes TIFF and names TIFF in its refusal. It
  runs the preview Operator after the header run. The library screen shows
  each image through its preview, and its picker and empty-library line
  name TIFF.
- `asset-library`: list entries and the Asset read gain `preview_url`. A new
  preview route serves the preview's bytes. The list, read and original
  routes cover originals only. A repeat upload is matched against originals
  only.
- `asset-page`: the page shows the image through its preview, or the
  original when there is none.
- `asset-facets`: `image.format` moves to schema version 2, with `TIFF`
  added to its allowed values. The header Operator reads TIFF width and
  height and gains the failure reason `the TIFF header could not be read`.

## Impact

- **`server/crates/plugin-abi`:** the public contract changes (ABI 4). An
  Operator declares the derived roles it makes, and the Asset context gains
  a way to propose a derived asset and write its bytes. `execute_operator`
  takes a host-supplied writer for those bytes.
- **`server/crates/core`:** a new migration adds the asset origin and the
  lineage table. `run_operator` stages, publishes and commits a derived
  asset with the run's outcome. The asset queries filter to originals, the
  repeat-upload lookup matches originals only, and a new query finds a
  version's preview.
- **`server/crates/packs/image`:** TIFF recognition, TIFF header reading
  and the new preview Operator. New dependencies: `tiff` (TIFF decoding)
  and `jpeg-encoder` (JPEG writing), both pure Rust. The pack still depends
  on `plugin-abi` only.
- **`server/crates/server`:** upload runs the preview Operator. The Asset
  routes add `preview_url` and the preview route, and the 415 message names
  TIFF.
- **`web/`:** the Library and asset page show the preview and fall back to
  the original. The picker accepts `image/tiff`, and the upload refusal and
  empty-library text name TIFF. No new design token.
- **Docs:** a new ADR records how Operator output becomes a derived asset
  with lineage, the open question that ADR-003 left for the Operator
  contract.
- **Cost:** every new upload stores a second object, its preview (D1), and
  a large TIFF makes the upload answer later (D7).
