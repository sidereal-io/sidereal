<!-- Plain language: Lead with the problem and resulting behavior. Give a reader enough context to decide whether the scope solves that problem. Keep the required headings. -->

## Why

Astrophotographers export finished images as TIFF, but upload refuses TIFF,
and browsers could not show one anyway. Nova needs to upload a TIFF and see
it in the library and on its page. A preview (a browser-ready copy of the
image, at full resolution) solves this. Making it is also the first time an
Operator produces a new asset. So this change also sets how a derived asset
and its lineage link are recorded, which later outputs such as stacks and
thumbnails will reuse.

A third format also shows where the image pack's format knowledge leaks.
The server and the web each repeat the pack's list of formats in their own
words, and the header Operator reads again the signature the Source already
read. This change makes the pack the one owner of its formats, and lets an
Operator learn a version's format from the media type the Source recorded
(D13, D14, D15).

## What Changes

- **Upload accepts TIFF.** The manual-upload Source recognizes classic TIFF
  and BigTIFF, in either byte order, from the file's leading bytes, with the
  media type `image/tiff` (D4).
- **One module per format in the image pack.** PNG, JPEG and TIFF each live
  in their own module behind an internal `ImageFormat` trait, and one
  `FORMATS` table drives recognition, registration and the refusal message.
  The reader of candidate bytes moves to a format-agnostic module. The
  Source reads as many leading bytes as the formats need (D13).
- **The pack's refusal reaches Nova.** The 415 answer carries the Source's
  own reason, `only PNG, JPEG and TIFF images are supported`. The web names
  no formats: a refused file's line shows the server's reason, the picker
  offers any file, and the empty library asks for an image (D14).
- **Operators see the media type and declare what they accept.** The Asset
  context gives an Operator the version's media type. Each Operator declares
  the media types it accepts, and core runs an Operator only on an
  original's version whose media type it accepts. Otherwise core records no
  run. No Operator accepts a derived version, so no Operator runs on a
  preview (D9, D15).
- **The header Operator reads TIFF.** It picks its reader from the media
  type and records the width and height of the first image in a TIFF. The
  asset page shows `Format TIFF` from `image/tiff` (D16). No format facet is
  added.
- **The plugin contract lets an Operator propose a derived asset.** An
  Operator that declares it makes previews can propose one new asset per
  run and stream its bytes through the Asset context. With the media type
  and the accepted media types, the plugin ABI version goes from 2 to 3.
  Core still mints every id and computes every hash.
- **Core records derived assets and lineage.** Core stores the Operator's
  output as a new asset and version, marks the asset as derived, and records
  a lineage link from the new version to the version it was made from, with
  the run that made it. A failed or refused run commits no asset.
- **The image pack adds a preview Operator.** It decodes PNG, JPEG and the
  supported TIFF types, and writes an 8-bit JPEG at quality 90 (D3). 16-bit
  and decimal values are scaled straight to 8-bit, with no brightening (D5).
  A TIFF it cannot decode, or an image whose pixels and decoder scratch space together need
  more than 512 MiB, fails the run with a reason (D4, D6).
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

- `asset-facets`: the Asset context gives an Operator the version's media
  type; an Operator declares the media types it accepts, and core runs it
  only on an original's version it accepts; an Operator run proposes no
  Asset as a Source does, only a preview. The header Operator picks its
  reader from the media type, reads TIFF width and height, and gains the
  failure reason `the TIFF header could not be read`; any failed read of the
  stored file fails as a storage problem.
- `image-upload`: upload recognizes TIFF, and its 415 answer carries the
  Source's reason. Upload runs the preview Operator after the header run.
  The library screen shows each image through its preview, shows the
  server's reason for a refused format, offers any file in its picker and
  names no format when it is empty.
- `asset-library`: list entries and the Asset read gain `preview_url`. A new
  preview route serves the preview's bytes. The list, read and original
  routes cover originals only. A repeat upload is matched against originals
  only.
- `asset-page`: the page shows the image through its preview, or the
  original when there is none, and names `image/tiff` as `TIFF`.

## Impact

- **`server/crates/plugin-abi`:** the public contract changes (ABI 3). An
  Operator declares the media types it accepts and whether it makes
  previews. The Asset context gives the version's media type, and gains a
  way to propose a derived asset and write its bytes. `execute_operator`
  takes the version's media type and a host-supplied writer for those
  bytes.
- **`server/crates/core`:** migration 6 adds the asset origin and the
  lineage table. `run_operator` checks what the Operator accepts before it
  records a run, and stages, publishes and commits a derived asset with the
  run's outcome. The asset queries filter to originals, the repeat-upload
  lookup matches originals only, and a new query finds a version's preview.
- **`server/crates/packs/image`:** split into `io.rs`, a `formats/` module
  with one file per format and its tests, the header Operator, and the new
  preview Operator. New dependencies: `tiff` (TIFF decoding) and
  `jpeg-encoder` (JPEG writing), both pure Rust. The pack still depends on
  `plugin-abi` only.
- **`server/crates/server`:** upload runs the preview Operator and passes
  the Source's refusal reason into its 415 answer. The Asset routes add
  `preview_url` and the preview route.
- **`web/`:** the Library and asset page show the preview and fall back to
  the original. The asset page names `image/tiff` as `TIFF`. The upload
  screen shows the server's refusal reason, and its picker and empty-library
  text name no format. No new design token.
- **Docs:** two new ADRs. ADR-018 records how Operator output becomes a
  derived asset with lineage, the open question that ADR-003 left for the
  Operator contract. ADR-019 records that Operators declare the media types
  they accept.
- **Cost:** every new upload stores a second object, its preview (D1), and
  a large TIFF makes the upload answer later (D7).
