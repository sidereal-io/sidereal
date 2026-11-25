<!-- Plain language: Help the next implementer find the approach, understand the choices and act on the plan. Explain necessary technical terms. Put each risk beside its mitigation and each open question beside the decision it affects. -->

## Context

- **Contract.** `server/crates/plugin-abi/src/lib.rs` is at `ABI_VERSION` 3.
  `AssetContext` has a private `RunKind`. `propose_asset` works only in a
  Source run, and `propose_facet` only in an Operator run. A broken rule sets
  `broken`, which turns the run into `ExecutionError::Failed`.
  `Registry::execute_operator(id, candidate, log, cancelled)` returns
  `FacetProposals`, which holds the producer and the grants from the
  Operator's registration. `RegisteredOperator` keeps `id`, `version` and
  `writes` from registration.
- **Core runs.** `server/crates/core/src/operations.rs` `run_operator` inserts a
  `running` row, runs the Operator in `spawn_blocking` over `ObjectBytes`,
  checks the proposals in `check`, then `store` writes the schema rows, the
  values and `succeeded` in one transaction. `record_failure` updates only a
  `running` row. A panic in the blocking task becomes the reason
  `the operator stopped`.
- **Assets.** `server/crates/core/src/assets.rs` selects every Asset joined to
  its current version. `find_or_create_asset` takes an advisory lock on the
  hash, matches any version's `content_hash`, and inserts an Asset and its
  first version with a deferred foreign key. `asset_versions` has
  `source_id` and `source_version`, both `NOT NULL`.
- **Storage.** `ObjectStore::begin()` gives a `Staging` that refuses more
  than `MAX_UPLOAD_BYTES` (100 MiB) with `StageError::TooLarge`.
  `finish()` hashes the bytes into a `Staged`, and `publish()` makes the
  object durable before any record (ADR-011). A dropped `Staging` or `Staged`
  removes its temporary file.
- **Migrations.** `server/crates/core/migrations/` holds 1 to 6. Each has a
  generated baseline `server/crates/core/schema/<n>.json`, made by
  `mise run db-schema` and listed in `baselines()` in
  `core/src/database/startup.rs`. Startup tests count the migrations.
- **Image pack.** `server/crates/packs/image` holds the manual-upload Source
  (PNG and JPEG signatures), `image.measure-file-size`, and
  `image.read-header` in `header.rs`. `header.rs` has `CandidateReader`, a
  `BufRead + Seek` view of the candidate built on 64 KiB `read_candidate`
  calls. The pack depends on `plugin-abi`, `png` 0.18 and `zune-jpeg` 0.5.
  `tests/memory.rs` counts the bytes a run allocates.
- **Server.** `server/crates/server/src/assets.rs` runs `upload_operators`
  (`FILE_SIZE`, `READ_HEADER`) in order after a new Asset is saved.
  `entry()` builds the JSON entry. `original()` streams an object with
  `nosniff` and `no-store`, and ends the body early when a read fails.
- **Web.** `web/src/entry.ts` `toEntry` accepts an entry only when
  `original_url` is exactly `/api/assets/<id>/original`. `Library.tsx` and
  `AssetPage.tsx` show `originalUrl` in an `<img>`, with a broken-image
  message on `error`. `upload.ts` holds the refusal texts.
- **Crates checked for this design.** `tiff` 0.11 decodes classic and
  BigTIFF in both byte orders, with optional `lzw` and `deflate` features;
  PackBits needs no feature. Its `Limits` caps the decoding buffer.
  `jpeg-encoder` 0.7 writes a baseline JPEG from an `ImageBuffer` trait that
  asks for one row at a time, and it takes width and height as `u16`.
  zune-jpeg keeps whether a JPEG is progressive private.

## Goals / Non-Goals

**Goals:**

- One general way for an Operator to output a derived asset with lineage,
  used first by previews. Later outputs (thumbnails, stacks) add a role, not
  a new mechanism.
- A preview Operator in `packs/image` that holds no more memory than the
  decoded pixels plus a small, fixed amount.
- The web interface shows a preview with no change to its layout or tokens.

**Non-Goals:**

- Backfilling previews, or a way to make one by hand (D8).
- Thumbnails, a tile pyramid or resizing.
- Brightening or stretching (D5), colour management, or applying EXIF or
  TIFF orientation.
- A general lineage API or a lineage view. Only the preview link is read.
- Running Operators over a preview (D9).

## Test Seams

Each layer has a seam that already exists. Every one is used, because each
crate's behavior is checked at its own boundary.

- **Primary test seam:** the server's HTTP tests in
  `server/crates/server/tests/assets.rs`, which upload through the router
  against PostgreSQL and read the list, the Asset, the original and now the
  preview. They prove the upload journey, `preview_url`, the preview route,
  originals-only listing and repeat-upload matching.
- **Existing seams reused:**
  - `plugin-abi` unit tests in `src/lib.rs`, for the context's rules on
    proposing a preview and writing its bytes.
  - Core's run tests under `core/src/operations/` with the `fixture.rs`
    database and `Scripted` Operators, for core's checks, the derived-asset
    commit, lineage, cancellation and the newest preview.
  - Core's startup tests in `core/src/database/startup/tests.rs`, for
    migration 7.
  - The image pack's tests in `packs/image/tests/`, which run
    `Registry::execute_operator` over in-memory bytes. New files
    `tiff_header.rs` and `preview.rs` follow `header.rs`, and `memory.rs`
    gains preview cases.
  - The web unit tests `Library.test.tsx`, `AssetPage.test.tsx` and
    `upload.test.ts`.

## Scrutiny Tier & Blast Radius

- **Tier:** high
- **Blast radius:** the public plugin contract (ABI 3 to 4), a database
  migration, core's run and asset queries, the image pack, the Asset routes
  and two web screens. Decoders read files that people upload, so memory and
  failure handling are part of the trust boundary.

## Decisions

### 1. The contract: an Operator declares previews and streams their bytes

- `Operator` gains a provided method `fn makes_previews(&self) -> bool`,
  which returns `false`. The Registry reads it once, at registration, into
  `RegisteredOperator`, as it reads `writes`. Adding a provided method keeps
  every existing Operator compiling.
- `AssetContext` gains, for Operator runs only:
  - `propose_preview(&mut self, proposal: AssetProposal) -> Result<(), ContextError>`,
    reusing `AssetProposal`'s kind and media-type checks;
  - `write_preview(&mut self, bytes: &[u8]) -> Result<(), ContextError>`.
  A second proposal, or a write before a proposal, sets `broken` and returns
  an error. A Source run gets `ContextError::NotAllowed` and `broken`, as an
  Operator calling `propose_asset` does today. A new
  `ContextError::Write` reports a write the host refused.
- The host supplies the destination: a new public trait
  `Output { fn write(&mut self, bytes: &[u8]) -> std::io::Result<()>; }`.
  `execute_operator` takes `output: &mut dyn Output` after `candidate`.
  `execute_source` is unchanged.
- `FacetProposals` becomes `OperatorProposals`. It keeps `producer_id`,
  `producer_version`, `grants` and `proposals`, and adds `makes_previews`
  (from registration), `preview: Option<AssetProposal>` and
  `preview_bytes: u64` (the count the context passed on).
- `ABI_VERSION` becomes 4, with a doc line.

*Alternatives:* the Operator returns the preview as a `Vec<u8>`. That holds
the whole JPEG in memory as well as the pixels, and puts no bound on it until
the run ends. A general `propose_output(role, …)` names roles before a second
one exists; `propose_preview` is narrower and can grow into it with ABI 5.
This decision gets an ADR, with decision 2.

### 2. Core records a derived asset and its lineage in the run's transaction

- Migration `7_derived_assets.sql`:
  - `public.assets` gains `origin TEXT NOT NULL DEFAULT 'source'
    CHECK (origin IN ('source', 'derived'))`. The default fills existing
    rows, then the migration drops it, so every insert names the origin.
  - New table `public.asset_lineage (output_version_id, input_version_id,
    role, run_id)`: both versions reference `asset_versions (id)`, `run_id`
    references `operation_runs (id)`, `role TEXT NOT NULL CHECK (role IN
    ('preview'))`, primary key `(output_version_id, input_version_id)`, a
    check that the two versions differ, and an index on
    `(input_version_id, role)`.
- `run_operator` gives the Operator a `StagingOutput`. On the first write it
  calls `ObjectStore::begin()`. It passes each write to `Staging::write` and
  remembers a `TooLarge` refusal. All of this runs in the existing
  `spawn_blocking` task.
- After the run, core checks the preview with the facet proposals, in
  `check`. The run fails, and the staging is dropped, when: a preview came
  from an Operator with `makes_previews` false (`operator <id> does not make
  previews`); the context reported more than one proposal or a write with
  none (the context's own reason); the media type is not `image/jpeg` or
  `image/png` (`a preview cannot be <media type>`); no byte was written
  (`the preview is empty`); or staging refused a write
  (`the preview is larger than 100 MiB`, which takes the place of the
  Operator's reason).
- After the cancellation check, core calls `finish()` and `publish()` on a
  blocking worker. Then `store` gains, inside its transaction and before the
  run is marked `succeeded`: read the input version's Asset name; insert the
  derived Asset (`origin` `derived`, the proposed kind, that name); insert
  its version (core's hash and size, the proposed media type, the Operator's
  id and version in `source_id` and `source_version`); insert the lineage row
  with role `preview` and the run id. A failed publish records the run
  `failed` with `the preview could not be stored`.
- `source_id` and `source_version` keep their names and now hold the
  producer: a Source for an original, an Operator for a derived asset.
  `origin` says which.

*Alternatives:* a side table of "renditions" attached to a version, with no
Asset. It is simpler to serve, but ADR-003 says derived output is a separate
Asset, and the glossary's Preview is a derived asset. A new version of the
original Asset is ruled out by ADR-003: the preview is not the original's
bytes. This decision gets an ADR.

### 3. Asset queries cover originals only, and find the newest preview

- `select()` in `assets.rs` gains `WHERE a.origin = 'source'` and a
  `preview` column from a lateral subquery: the derived version linked to the
  current version with role `preview`, by the run with the latest
  `ended_at`, then the highest run id. `AssetRecord` gains
  `has_preview: bool`. `Database::assets()` and `Database::asset(id)` both
  use it, so a derived id reads as missing.
- A new `Database::preview(asset_id) -> Option<VersionRecord>` gives the
  preview version of an original's current version, for the preview route.
- `find_or_create_asset` matches `content_hash` only on versions whose Asset
  has `origin = 'source'`. The advisory lock stays as it is. Its insert names
  `origin` `source`.

### 4. The image pack: TIFF signatures and one small TIFF directory reader

- `Format` gains `Tiff`, named `TIFF`, with media type `image/tiff`. Its
  signatures are `II*\0`, `MM\0*`, `II+\0` and `MM\0+`. `UNSUPPORTED`
  becomes `only PNG, JPEG and TIFF images are supported`.
- A new module `tiff_ifd.rs` reads the 8-byte (classic) or 16-byte
  (BigTIFF) header and the first image directory through `CandidateReader`.
  It returns the values of the tags it needs: width (256), length (257),
  bits per sample (258), compression (259), photometric interpretation
  (262), samples per pixel (277), planar configuration (284), predictor
  (317), extra samples (338) and sample format (339). It reads at most 4,096
  directory entries and refuses a value array of more than 64 entries, so a
  run reads a bounded number of bytes.
- `header.rs` uses it for TIFF and proposes the width and length, whatever
  the other tags hold. Its failure reason is
  `the TIFF header could not be read`. `CandidateReader` moves to its own
  module, `reader.rs`, so the preview Operator can use it too.
- `image.format` is declared at version 2 with `PNG`, `JPEG`, `TIFF`. Core
  keeps the version-1 row and its values, as ADR-018 says.

*Alternative:* `tiff::decoder::Decoder::new` for the header. It reads the
first directory too, but it refuses some color types and compressions while
it sets up, and the header run must succeed for every TIFF (D4).

### 5. The preview Operator decodes into one buffer and encodes row by row

- New module `preview.rs` registers `image.make-preview`
  (`MAKE_PREVIEW`), which makes previews and writes no facets.
- **Order of work.** Find the format by signature. Read the header: IHDR for
  PNG, the frame header for JPEG through zune-jpeg's `decode_headers` with
  its 8 MiB read limit, `tiff_ifd` for TIFF. For TIFF, refuse every type that
  the spec does not list before any decoding. Check that each side is at most
  65,535. Compute the decoded size as the spec says, and fail over
  536,870,912 bytes. Only then decode.
- **Progressive JPEG.** zune-jpeg does not say whether a JPEG is progressive,
  so the pack walks the markers to the first frame header itself and reads
  its kind (`0xC2` is progressive), under the same 8 MiB limit.
- **Decoders.** PNG: the `png` crate with `Transformations::EXPAND`, keeping
  16-bit samples, and `png::Limits { bytes }` set to the limit. JPEG:
  zune-jpeg `decode_into` a buffer of `output_buffer_size`, with output
  colour space Luma for one component and RGB otherwise. TIFF: the `tiff`
  crate with `default-features = false, features = ["lzw", "deflate"]` and
  `Limits { decoding_buffer_size: 536_870_912, .. }`, through `read_image`.
- **Conversion.** A `Rows` type implements `jpeg_encoder::ImageBuffer` over
  the decoded buffer. Its `fill_buffers` turns one row into 8-bit Luma or RGB
  as the spec says, so no second full-size buffer exists.
- **Encoding.** `jpeg_encoder::Encoder` at quality 90, baseline, with 4:4:4
  sampling (no chroma subsampling, so star colours keep their detail) and
  the standard Huffman tables, so the encoder streams. Its writer collects up
  to 64 KiB and passes each chunk to `context.write_preview`.
- **Failure reasons** are the spec's. A decoder error, a short read and a
  `tiff` limit error all become `the <FORMAT> image could not be decoded`.
- The pack's dependencies become `plugin-abi`, `png`, `zune-jpeg`, `tiff` and
  `jpeg-encoder`. `check-arch.sh` still holds.

*Alternatives:* the `image` crate decodes and encodes all three formats, but
it brings many codecs, always makes a second 8-bit buffer, and hides the
limits we need. 4:2:0 sampling makes smaller files but softens coloured
stars.

### 6. The server runs one preview at a time

- `upload_operators` becomes `FILE_SIZE`, `READ_HEADER`, `MAKE_PREVIEW`.
- `Library` holds a `tokio::sync::Semaphore` with one permit. `save` takes it
  around the preview run only, so two uploads at once never decode two
  512 MiB images at once. The other runs do not wait for it.
- `save` returns the record with `has_preview` set to whether the preview run
  succeeded. Core commits the lineage in the same transaction that records
  `succeeded`, so no second read is needed.
- `entry()` adds `preview_url`: `/api/assets/<id>/preview`, or `null`.
- The route `GET /api/assets/{id}/preview` shares one helper with
  `original()` that streams an object with its media type, `nosniff`,
  `no-store`, and the early end on a failed read.
- The 415 message becomes
  `unsupported format: only PNG, JPEG and TIFF images are supported`.

### 7. The web shows the preview and falls back to the original

- `toEntry` accepts `preview_url` only when it is `null` or exactly
  `/api/assets/<id>/preview`, and gives `Entry.imageUrl`, the preview or else
  the original. `AssetPage.tsx`'s `assetOf` does the same.
- `Library.tsx` and `AssetPage.tsx` use `imageUrl` in their `<img>`. The
  picker's `accept` becomes `image/png,image/jpeg,image/tiff`, and the
  empty-library line names TIFF. `upload.ts` changes the 415 and 400 texts.
- **Tokens:** none. The screens' layout and styles do not change.

## Risks / Trade-offs

- [A decoder holds more than the decoded pixels: PNG row buffers, zune-jpeg's
  internal state, `tiff`'s intermediate buffer] → `memory.rs` gains preview
  cases that measure the peak for a 16-bit RGB TIFF, a progressive JPEG and an
  RGBA PNG, and assert it is at most the decoded size plus 64 MiB. Set
  `tiff`'s `intermediate_buffer_size` if the TIFF case needs it.
- [Several uploads at once each decode a large image] → decision 6 allows one
  preview run at a time per server.
- [A large TIFF makes the upload's answer late (D7)] → the web keeps the
  file's line at `Uploading` until the answer comes; no timeout is added.
- [Every new upload stores a second object (D1)] → accepted. A JPEG at
  quality 90 is usually much smaller than a 16-bit TIFF.
- [A JPEG original loses a little quality in its preview] → accepted:
  quality 90 with 4:4:4 sampling.
- [A wide-gamut or tagged TIFF (Adobe RGB, ProPhoto) looks less saturated,
  because the preview carries no colour profile] → out of scope. A follow-up
  can copy the ICC profile into the JPEG; `jpeg-encoder` can write one.
- [A phone JPEG with an EXIF rotation shows unrotated, where the browser
  rotated the original before] → the spec says so. Astro images rarely carry
  a rotation; a follow-up can apply it.
- [A panic in a decoder] → `spawn_blocking` already turns a panic into a
  failed run, `the operator stopped`.
- [The contract changes for third-party packs (ABI 4)] → it is unfrozen
  until M2. The new `Operator` method has a default, and only hosts call
  `execute_operator`.

## Migration & Rollback

- **Forward:** migration 7 adds `assets.origin`, filled with `source` for
  every existing row, and the empty `asset_lineage` table. Generate
  `core/schema/7.json` with `mise run db-schema`, add it to `baselines()`, and
  move the startup tests and `scripts/instance-verify.sh` from six to seven
  migrations. No data is backfilled (D8).
- **Rollback:** migrations are forward-only (ADR-004), and a server built
  before this change refuses a database with seven migrations. To roll back,
  restore the database backup taken before the upgrade. Preview objects left
  in storage are then unreferenced and harmless.
