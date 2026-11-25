<!-- Plain language: Help the next implementer find the approach, understand the choices and act on the plan. Explain necessary technical terms. Put each risk beside its mitigation and each open question beside the decision it affects. -->

## Context

- `server/crates/plugin-abi/src/lib.rs` has one value type. `FacetType` has
  only `Count`, and `FacetValue` has only `Integer(i64)`. `ABI_VERSION` is 2.
  This change does not touch `plugin-abi`.
- Each Asset version already has a `media_type`, such as `image/png`. The
  manual-upload Source in `server/crates/packs/image/src/lib.rs` sets it from
  the file's signature, not its name, and refuses bytes that are not PNG or
  JPEG. Core reads it into `AssetRecord.version.media_type`
  (`server/crates/core/src/assets.rs`). No response returns it yet.
- Under ADR-008 one pack owns a facet's schema, and only it may write the
  facet. A format facet owned by `packs/image` could therefore never hold
  FITS or XISF from `packs/astro` (D3).
- `server/crates/packs/image/src/lib.rs` has the Source and the file-size
  Operator `image.measure-file-size`. The pack depends on `plugin-abi` and no
  other Sidereal crate, and `server/scripts/check-arch.sh` enforces that. A
  pack may depend on crates from crates.io.
- `server/crates/server/src/assets.rs` runs `upload_operators` in order after
  a new Asset is saved. It logs `file-size run failed for asset …` and
  `file-size run not recorded for asset …` whatever Operator ran. `entry`
  builds the library list entry, which `read` and the upload response reuse.
- `web/src/AssetPage.tsx` accepts a facet only when its `value` is a safe
  whole number of zero or more, always prints `value unit`, and shows
  `No facts are recorded for this image.` when the server returns no facets.
- An Operator reads bytes only through `AssetContext::read_candidate`, at
  most 64 KiB per call, at any offset.
- The test fixtures `nebula.png` and `nebula.jpg` in
  `server/crates/server/tests/fixtures/` are both 32 by 24 pixels.

### State of the branch

The first build of this change added a choice value type to `plugin-abi`
(`FacetType::Choice`, `FacetValue::Text`, `ABI_VERSION` 3), the
`image.format` facet, migration `6_facet_allowed_values.sql` with its
baseline `core/schema/6.json`, ADR-018, and the code and tests that use
them. The pull request review found that a closed choice owned by one pack
cannot take formats from another (D3). The user removed all of it from this
change: a choice type comes back with its first real user, such as
`astro.frame_type`, and a forward-only migration with no user should not
reach `main`. ADR-018 is gone from `docs/decisions/`; tasks section 7
removes the code. The header reader, its 8 MiB JPEG limit, its memory tests
and the build-review fixes stay.

## Goals / Non-Goals

**Goals:**

- A header Operator in `packs/image` that reads the width and height of PNG
  and JPEG without decoding pixel data.
- Upload runs it after the file-size Operator, and a failure in one run
  leaves the other run's values in place.
- The asset page shows a `Format` line from the version's media type, then
  the facets, and leaves out an empty unit.

**Non-Goals:**

- A choice, text or other new value type in `plugin-abi`, or any change to
  the plugin contract.
- A migration, or any change to core.
- EXIF, colour profiles, TIFF, or applying EXIF orientation (D1, D8).
- Backfilling Assets saved before this change (D9), or showing failed runs on
  the page (D7).
- An order for facets other than by name (D10).
- `media_type` in the library list or the upload response (D11).

## Test Seams

- **Primary test seam:** the image pack's public contract, as
  `server/crates/packs/image/tests/file_size.rs` uses it. A test registers
  `ImagePack`, runs `Registry::execute_operator` over in-memory bytes, and
  asserts the proposals or the failure reason. This covers the PNG and JPEG
  header reader and every failure reason without a database.
- **Existing seams reused:**
  - `server/crates/server/tests/assets.rs` for upload, the log lines,
    `media_type` and the facet JSON, using the existing fixtures and PNGs
    built in the test.
  - `web/src/AssetPage.test.tsx` for the page's lines.
  - `web/e2e/upload.spec.ts` for Nova's journey.

## Scrutiny Tier & Blast Radius

- **Tier:** high
- **Blast radius:** the image pack and its first crates.io dependencies
  (`png` for PNG, `zune-jpeg` for JPEG), which parse untrusted bytes; the
  upload route's Operator list and log lines; the read of one Asset; and the
  asset page. No authorization or trust boundary changes, and no change to
  `plugin-abi`, core or the database schema.

## Decisions

### 1. The format comes from the media type, not a facet

The asset page shows the format from the current version's `media_type`.
The image pack declares no `image.format` facet, and its header Operator
proposes only the width and height. The media type already comes from the
file's bytes, so the page still never names a format from a file name.

- *Alternative:* `image.format`, a choice of `PNG` or `JPEG` owned by
  `packs/image`, with a new choice value type in `plugin-abi`. This was the
  first build. Under ADR-008 only the owning pack may write a facet, so
  `packs/astro` could never add FITS or XISF, and the choice type, its
  migration and its record would have no other user.
- *Alternative:* an open text facet `image.format`. It repeats what the
  media type already says, and it still belongs to one pack.
- *Alternative:* a format facet in each pack, such as `astro.format`. One
  fact would then live under several names, and the page would have to know
  them all.

### 2. Reading one Asset adds `media_type`

`read` in `server/crates/server/src/assets.rs` adds `"media_type":
record.version.media_type` to the `asset` object, beside `facets`. `entry`
does not change, so the library list and the upload response keep their
shape (D11). Core already reads the field; core does not change.

- *Alternative:* add it to `entry`, so every Asset entry carries it. Nothing
  else needs it yet, and the user chose the read only.

### 3. The image pack reads headers with `png` and `zune-jpeg`

The header Operator, `image.read-header`, reads the first 8 bytes and picks
the format from the signature, as the Source does. Both share one signature
function. Bytes that start with neither signature fail with `the image format
is not recognized`. A failed read of those first bytes is a storage problem,
not an unknown format, so it fails with `the stored file could not be read`.

The Operator does not parse PNG or JPEG itself. It uses two pure-Rust crates
from crates.io, the decoders that the `image` crate uses, called directly. It
names the format from the signature, so neither crate guesses from a file
name:

- **PNG:** [`png`](https://crates.io/crates/png) (0.18):
  `png::Decoder::new(reader)`, then `read_header_info()`, which gives the
  width and height. It reads the signature and the `IHDR` chunk and stops.
  It keeps no text, colour profile or EXIF data, and reserves no row of
  pixels.
- **JPEG:** [`zune-jpeg`](https://crates.io/crates/zune-jpeg) (0.5):
  `JpegDecoder::new_with_options(reader, options)`, then `decode_headers()`
  and `dimensions()`. `options` raises zune-jpeg's default limit of 16,384
  pixels per side to 65,535, the most a JPEG frame header can hold, as
  `image` does, so a large mosaic is not refused.

Each decoder reads the header and gives the width and height that the header
stores. Neither decodes pixel data or applies EXIF orientation (D8).

- **The reader.** Both decoders need `BufRead + Seek`. A small adapter in the
  pack, `CandidateReader`, wraps the `AssetContext`. It keeps the last two
  64 KiB windows (`AssetContext::MAX_READ`), and fills a window with one
  `read_candidate` call at the current offset. `seek` only moves the offset; a
  read fills a new window only when the offset lies outside both windows.
  zune-jpeg reads up to 64 KiB of a segment ahead and then seeks back to the
  segment's start, and the second window means that seek back needs no new
  read. A short read is not a failure: the adapter returns what it got, and
  only a read of 0 bytes means the end of the candidate. Before each fill it
  checks `is_cancelled`, and returns an I/O error when the run is cancelled.
- **The JPEG read limit.** For a JPEG, `CandidateReader` treats the candidate
  as if it ended after its first 8 MiB (8,388,608 bytes): it never asks
  for a byte past that point, and a read there returns 0 bytes. A JPEG whose
  header, through the end of its first start-of-scan segment, does not fit
  in the first 8 MiB therefore fails as a cut-off header. zune-jpeg reads
  that whole segment before `decode_headers()` returns, so the limit covers
  the segment, not only its marker. Camera metadata and the usual ICC
  profiles are far smaller; only an ICC profile or extended XMP of several
  MiB before the frame header is refused, which is rare. The limit is 8 MiB,
  not 32 MiB, to bound memory (see Memory). The PNG reader needs no limit,
  because it stops after `IHDR`.
- **The bound on reads.** When each `read_candidate` returns the whole window
  it asked for, as the server's file-backed candidate does for a regular
  file, a JPEG run makes at most n / 65,536 + c calls, where n is the
  smaller of the candidate's size and 8 MiB, and c is a small constant for
  seeks outside both windows. That is at most about 132 calls. A PNG run
  makes one call. A candidate that returns fewer bytes costs more calls, but
  gives the same answer. No cap on the number or length of segments
  applies.
- **Memory.** A run holds the two windows, 128 KiB, and what the decoder
  keeps from the header. The PNG decoder keeps only the `IHDR` fields.
  zune-jpeg keeps the last EXIF, XMP and IPTC segment, each under 64 KiB, and
  every ICC profile and HDR gain-map segment it passes before start-of-scan.
  For each ICC segment it keeps a 32-byte entry in a vector and a separate
  heap allocation for the segment's data, so the cost is not the bytes read.
  The worst case is a JPEG of the smallest ICC segments, 19 bytes each in
  the file: 8 MiB holds about 441,500 of them. The vector then grows to a
  capacity of 524,288 entries, 16 MiB, and up to 24 MiB while it grows; the
  1-byte allocations take about 14 MiB with allocator overhead. One header
  run therefore holds at most about 30 to 40 MiB, close to the 32 MiB budget
  for a run. Each upload in flight runs its header on its own blocking
  worker, so uploads at the same time add up. `packs/image/tests/memory.rs`
  counts the bytes a run asks the allocator for, at its peak: about 24 MiB
  for this JPEG, which it holds to 40 MiB, and about 83 KiB for a PNG of
  200,000 text chunks, which it holds under 1 MiB. The count leaves out the
  allocator's own overhead.
- **PNG.** The decoder checks the `IHDR` chunk's CRC-32 before it parses the
  chunk, so a damaged header fails rather than giving wrong dimensions. A
  first chunk that is not a 13-byte `IHDR`, or a header cut off early, also
  fails. Chunks after `IHDR` are never read.
- **JPEG.** zune-jpeg reads the markers through its first start-of-scan
  segment and
  skips the body of each segment it does not keep. It reads the frame
  headers of 8-bit baseline, extended and progressive JPEGs (`SOF0` to
  `SOF2`). It skips other frame kinds, such as lossless (`SOF3`), as unknown
  segments, so a JPEG with no supported frame header fails when it reaches
  start-of-scan. A frame header with a precision other than 8 bits, such as
  a 12-bit `SOF1`, fails too.
- **Failures.** Any error from a decoder, including one from the adapter,
  fails the run. The Operator reports the dimensions a header states. It
  does not check that the image data can be decoded, and the decoders do not
  check every field of a header: zune-jpeg, for example, stores a scan's
  Huffman table selectors without a range check. So a header the decoder
  accepts gives its dimensions even when a full decode would fail. The reason is `the PNG header could not be read` or `the
  JPEG header could not be read`, by signature. The Operator also fails a
  width or height of 0, and a PNG width or height above 2,147,483,647, the
  PNG limit, whatever the crate accepts. The Operator then proposes
  `image.width` and `image.height`. It proposes no format: the format it
  finds only picks the decoder and the failure reason (decision 1).

- *Alternative:* a hand-written PNG and JPEG header reader, about 80 lines.
  It adds no dependency, but the review found it needs its own bound on
  work, its own CRC-32 and its own structural checks, each a place to get
  wrong. `png` and `zune-jpeg` are the decoders the `image` crate uses, and
  #526 adds `image` to render TIFF previews over the same decoders.
- *Alternative:* `image` for PNG, with `max_alloc` at 32 MiB. `image` turns
  on text parsing, and `png` charges each text payload but not the entry it
  keeps, so a PNG of millions of tiny `tEXt` chunks still holds hundreds of
  MiB.
- *Alternative:* `image` for JPEG. It reads the whole candidate into memory
  first, so each JPEG upload in flight holds up to 100 MiB.
- *Alternative:* no JPEG read limit, with at most two header runs at once.
  Each run could still keep close to the file size, about 200 MiB in all,
  and a third upload would wait for its answer.
- *Alternative:* a 32 MiB JPEG read limit. Because zune-jpeg keeps an entry
  and an allocation per ICC segment, a JPEG of 19-byte ICC segments makes
  one run hold about 120 to 150 MiB within 32 MiB of reading.
- *Alternative:* libvips or exiftool. Both read headers well, but each adds
  a C or Perl dependency to the build and the Docker image.
- *Alternative:* the `imagesize` crate. It reads from a slice or a file, not
  a `Read + Seek`, so the Operator would still copy bytes out of the
  context first.

### 4. Upload runs both Operators and logs by Operator id

`Library::new` sets `upload_operators` to `[FILE_SIZE, READ_HEADER]`. Each
run is independent, as `save` already loops over the list and never stops on
a failure. The log lines become `<operator id> run failed for asset <id>:
<reason>` and `<operator id> run not recorded for asset <id>: <error>`. The
existing server tests that match `file-size run …` change to the new lines.

### 5. The asset page shows a `Format` line, then the facets

`assetOf` reads `asset.media_type`, and refuses an answer whose media type
is not a non-empty string, as it refuses a malformed facet: the page shows
`The image could not be loaded.` and Retry. A small map gives the line's
text: `image/png` is `PNG`, `image/jpeg` is `JPEG`, and any other media type
is shown as it is, as React text, so markup never runs (D4). `Details`
always shows the `Format` line first, with no unit, then one line per facet
in the server's order (D10). The no-facts line goes, because the list is
never empty (D12).

`Fact.value` stays a number: a facet still holds only counts. A line's unit
and the space before it render only when the unit is not empty, because the
`Format` line has none and a pack may declare a facet with no unit. The
`white-space: pre-wrap` rule on the value stays, so a media type the page
does not know shows exactly as the server returns it; its comment changes to
say so.

- *Alternative:* show the media type itself, `image/png`. It is accurate but
  reads as a technical label to Nova; the common name is what Nova knows.
- *Alternative:* keep the no-facts line for an Asset with no facets. The
  `Format` line would then sit beside a line that says no facts are recorded.

**Tokens added:** none. The `Format` line reuses the existing fact line and
its tokens.

## Risks / Trade-offs

- [A crafted file could make many small reads, such as a JPEG of 4-byte
  segments or fill bytes, or one of full-length segments that zune-jpeg
  reads ahead into and seeks back over] → The adapter reads in 64 KiB
  windows and keeps the last two, and reads no further than 8 MiB into a
  JPEG, so with full reads a run costs at most about 132 reads, whatever the
  layout. Task 3.3 tests the bound with both files.
- [A crafted PNG could make its header run allocate a lot of memory, such as
  millions of tiny text chunks or a colour profile that inflates to hundreds
  of MiB] → The PNG reader stops after `IHDR` and keeps nothing else. Task
  3.2 tests a PNG of 200,000 text chunks after its `IHDR`.
- [zune-jpeg keeps every ICC profile and HDR gain-map segment it passes
  before the start-of-scan marker, with no limit of its own, so a crafted
  JPEG made of such segments could make one header run hold close to its
  own size, and keeps a 32-byte entry and a heap allocation per ICC segment]
  → The reader stops at 8 MiB into a JPEG, so a run holds at most about 30
  to 40 MiB, the worst case being 19-byte ICC segments (see Memory). Task
  3.3 tests a JPEG with 9 MiB of ICC segments before its frame header, and
  one of 19-byte ICC segments that fills the limit.
- [The process stops after the Asset commits, for example when it runs out
  of memory, and the header run stays `running` with no outcome] → The
  Asset and its file are saved. #527 checks processing again at startup, as
  D9 says.
- [Some valid files fail the header run: a JPEG whose height is set by a
  later marker, a JPEG kind the decoder does not support (such as lossless,
  12-bit or arithmetic-coded), a JPEG whose first start-of-scan segment
  ends more than 8 MiB into the file, or an Apple-optimized PNG whose first
  chunk is `CgBI`] → The run fails with a clear reason and the file size is
  still recorded. The page still shows the format from the media type. These
  files are rare in astrophotography output, and #527 shows processing
  failures.
- [A new `png` or `zune-jpeg` release changes which files it accepts] →
  `Cargo.lock` pins the versions, the header tests cover each case the specs
  name, and Dependabot updates arrive as pull requests that run those tests.
- [The format is not a fact a selector or search can use, because it is not
  a facet] → No selector or search exists yet. When one needs the format, it
  can read the version's media type, which every pack's Source sets.
- [The page's map from media type to name covers only PNG and JPEG] → Any
  other media type shows as it is, so a new format is never hidden; #526 and
  `packs/astro` add names as they add formats.
- [The upload answers later, because it now waits for two runs] → A PNG
  header run reads one 64 KiB window. A JPEG header run reads through its
  first start-of-scan segment, which in the usual file lies in the first
  window.

## Migration & Rollback

- **Deploy:** no migration. The branch's migration 6 and its baseline are
  removed before merge (task 7.2), so `main` never holds them and a database
  keeps five migration records.
- **Data:** no backfill (D9). Assets saved before this change keep only the
  facts they have, and show their format from the media type they already
  hold.
- **Rollback:** deploy the previous build. Nothing in the database changes,
  so an older server reads the same rows; only the stored width and height
  values of new uploads stay behind, under schemas it does not declare,
  which it reads with their kept label and unit.
