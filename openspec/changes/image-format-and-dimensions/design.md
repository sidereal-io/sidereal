<!-- Plain language: Help the next implementer find the approach, understand the choices and act on the plan. Explain necessary technical terms. Put each risk beside its mitigation and each open question beside the decision it affects. -->

## Context

- `server/crates/plugin-abi/src/lib.rs` has one value type. `FacetType` has
  only `Count`, and `FacetValue` has only `Integer(i64)`. Both derive `Copy`,
  and `ABI_VERSION` is 2. `unstorable_facet` checks a schema when its pack
  registers.
- `server/crates/core/src/operations.rs` runs an Operator, checks every
  proposal in `check`, and stores values in `store`. `store` keeps one
  `facet_schemas` row per name and version, and refuses a run when the kept
  row (owner, label, value type, unit, scope) differs from the running
  declaration. `Database::facets` reads values back and treats anything but a
  count in range as a damaged database. `FacetEntry.value` is an `i64`.
- `facet_schemas` (migration `5_facets.sql`) has no column for allowed
  values. ADR-017 keeps values in a JSONB column, so a text value needs no
  change to `facet_values`.
- `server/crates/packs/image/src/lib.rs` has the manual-upload Source, which
  refuses bytes that are not PNG or JPEG, and the file-size Operator
  `image.measure-file-size`. The pack depends on `plugin-abi` and no other
  Sidereal crate, and `server/scripts/check-arch.sh` enforces that. A pack
  may depend on crates from crates.io.
- `server/crates/server/src/assets.rs` runs `upload_operators` in order after
  a new Asset is saved. It logs `file-size run failed for asset …` and
  `file-size run not recorded for asset …` whatever Operator ran.
- `web/src/AssetPage.tsx` accepts a facet only when its `value` is a safe
  whole number of zero or more, and always prints `value unit`.
- An Operator reads bytes only through `AssetContext::read_candidate`, at
  most 64 KiB per call, at any offset.
- The test fixtures `nebula.png` and `nebula.jpg` in
  `server/crates/server/tests/fixtures/` are both 32 by 24 pixels.

## Goals / Non-Goals

**Goals:**

- A choice value type that any pack can declare through `plugin-abi`, with
  core checking every proposal against the schema's allowed values.
- A header Operator in `packs/image` that reads the format, width and height
  of PNG and JPEG without decoding pixel data.
- Upload runs it after the file-size Operator, and a failure in one run
  leaves the other run's values in place.
- The asset page shows count and choice values, and leaves out an empty unit.

**Non-Goals:**

- Free text, decimal or other value types.
- EXIF, colour profiles, TIFF, or applying EXIF orientation (D1, D8).
- Backfilling Assets saved before this change (D9), or showing failed runs on
  the page (D7).
- An order for facets other than by name (D10).
- A `type` field in the facet JSON: a client tells a count from a choice by
  the JSON type of `value` (decision 6).

## Test Seams

- **Primary test seam:** the image pack's public contract, as
  `server/crates/packs/image/tests/file_size.rs` uses it. A test registers
  `ImagePack`, runs `Registry::execute_operator` over in-memory bytes, and
  asserts the proposals or the failure reason. This covers the PNG and JPEG
  header reader and every failure reason without a database.
- **Existing seams reused:**
  - `plugin-abi`'s unit tests for the choice declaration checks.
  - `server/crates/core/src/operations/` tests (`refuses.rs`, `stores.rs`,
    `fixture.rs`) for the proposal check, the kept schema comparison and the
    facet read, against a test database.
  - `server/crates/server/tests/assets.rs` for upload, the log lines and the
    facet JSON, using the existing fixtures and PNGs built in the test.
  - `web/src/AssetPage.test.tsx` for the page's lines.

## Scrutiny Tier & Blast Radius

- **Tier:** high
- **Blast radius:** the public plugin contract (`plugin-abi`), core's facet
  check, store and read paths, one forward-only migration on
  `facet_schemas`, the image pack and its first crates.io dependencies
  (`png` for PNG, `zune-jpeg` for JPEG), the upload route's Operator list
  and log lines, the asset JSON,
  and the asset page. No authorization or trust
  boundary changes.

## Decisions

### 1. The choice type lives on `FacetType`, and the text on `FacetValue`

`FacetType` gains `Choice(Vec<String>)`, holding the allowed values in their
declared order. `FacetValue` gains `Text(String)`. Neither enum stays `Copy`;
each keeps `Clone`, `Debug`, `PartialEq` and `Eq`, so `FacetSchema` still
compares in tests. `ABI_VERSION` rises to 3, with a doc line that says
facets can be choices.

- *Alternative:* a separate `allowed_values` field on `FacetSchema`. It would
  sit empty on every count and let a count carry allowed values by mistake.
  Holding them inside the variant makes that state impossible.
- *Alternative:* `Choice(&'static [&'static str])`, to keep `Copy`. A third-
  party pack loaded later could not build its list at run time, and the
  contract is meant for those packs.

The value variant is named for its shape, as `Integer` is, so a later text
type can reuse `Text`.

### 2. The Registry checks a choice when the pack registers

`unstorable_facet` refuses a choice with no allowed values, an empty allowed
value, a value with U+0000, or a value listed twice. The error is
`RegistryError::InvalidFacet` with a reason, as for other declaration
faults, so the error names the facet. A refused pack adds nothing, as today.

### 3. Core keeps the allowed values with the schema version

A forward-only migration, `6_facet_allowed_values.sql`, adds
`allowed_values TEXT[]` to `facet_schemas`. It is `NULL` for a count and holds
one or more values for a choice. A check constraint enforces that the column
is `NULL` when `value_type` is not `'choice'`, and is not `NULL` and holds at
least one value (`cardinality(allowed_values) >= 1`) when it is. Existing
rows are counts with `NULL`, so they stay valid. `type_text` returns `"choice"` for a choice. `store` writes the
allowed values with a new schema row, and adds them to `KeptSchema`, so a
kept version with other allowed values, or the same ones in another order,
refuses the run with the existing reason. Order counts because the
comparison is the simplest exact one, and a pack has no reason to reorder.

- *Alternative:* no column, and rely on the running declaration. A value
  could then be read back under allowed values it was never checked against,
  and a changed list would not refuse the run.
- *Alternative:* a general `JSONB` column for any type's parameters. It would
  save a migration for a future parameterized type, but loses the database's
  check on the shape, and no second such type exists yet.

ADR-018 records this decision, because it changes what ADR-017 expected: the
schema table now carries one type's parameters.

### 4. Core checks and reads a choice by exact match

`check` accepts `(Choice(allowed), Text(t))` only when `allowed` contains
`t`, letter case included. Any other pairing of type and value, such as text
for a count, is refused with a reason that names the facet. `json` writes a
`Text` value as a JSON string.

`Database::facets` also selects `allowed_values`. It reads a choice value
only when it is a JSON string in the kept allowed values, and otherwise
returns `DatabaseError::Schema`, as it does for a count out of range.
`FacetEntry.value` becomes a `FacetValue`, so the server maps `Integer` to a
JSON number and `Text` to a JSON string.

### 5. The image pack reads headers with `png` and `zune-jpeg`

The header Operator, `image.read-header`, reads the first 8 bytes and picks
the format from the signature, as the Source does. Both share one signature
function. Bytes that start with neither signature fail with `the image format
is not recognized`.

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
  counts the bytes a run asks the allocator for, at its peak: about 17 MiB
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
  `image.format`, `image.width` and `image.height`.

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

### 6. The facet JSON keeps its shape

Each facet entry keeps `name`, `label`, `value` and `unit`. A choice's
`value` is a JSON string, and a facet with no unit has `"unit": ""`. The web
page tells a count from a choice by `typeof value`.

- *Alternative:* add a `type` field. It would let a client check the value
  against its type, but no client needs that yet, and it can be added later
  without breaking readers.

### 7. Upload runs both Operators and logs by Operator id

`Library::new` sets `upload_operators` to `[FILE_SIZE, READ_HEADER]`. Each
run is independent, as `save` already loops over the list and never stops on
a failure. The log lines become `<operator id> run failed for asset <id>:
<reason>` and `<operator id> run not recorded for asset <id>: <error>`. The
existing server tests that match `file-size run …` change to the new lines.

### 8. The asset page shows text values and empty units

`Fact.value` becomes `number | string`. `facts()` accepts a safe whole number
of zero or more, or a non-empty string, and refuses anything else, so the
page still shows `The image could not be loaded.` for a malformed answer. A
number shows through `numbers.format`; a string shows as React text. The unit
and the space before it render only when the unit is not empty. No CSS
changes: the value and unit stay in the existing `dd`, which is already in
the mono face.

**Tokens added:** none. The page reuses the existing fact line and its
tokens.

## Risks / Trade-offs

- [Making `FacetType` and `FacetValue` non-`Copy` breaks code that copies
  them, such as `match (declared.schema.value_type, proposal.value)` in
  `check`] → Match on references. Only first-party code uses the contract,
  and the build finds every use.
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
  ends more than 8 MiB into the file, or an Apple-optimized PNG whose first chunk is
  `CgBI`] → The run
  fails with a clear reason and the file size is still recorded. These files
  are rare in astrophotography output, and #527 shows processing failures.
- [A new `png` or `zune-jpeg` release changes which files it accepts] →
  `Cargo.lock` pins the versions, the header tests cover each case the specs
  name, and Dependabot updates arrive as pull requests that run those tests.
- [Adding `PNG` or `JPEG` as display text makes it the stored value too, so
  renaming it later needs a new schema version] → That is the cost D3
  accepted. The values are the formats' common names and are not expected to
  change.
- [The upload answers later, because it now waits for two runs] → A PNG
  header run reads one 64 KiB window. A JPEG header run reads through its
  first start-of-scan segment, which in the usual file lies in the first
  window.

## Migration & Rollback

- **Deploy:** the server applies `6_facet_allowed_values.sql` at start, as it
  applies every migration. It adds one nullable column and one check
  constraint to `facet_schemas`. Existing rows are counts and keep `NULL`.
  The migration needs its schema baseline, `core/schema/6.json`, made by
  `mise run db-schema` and added to `baselines()` in
  `core/src/database/startup.rs`. The startup tests and
  `scripts/instance-verify.sh` then expect six migration records.
- **Data:** no backfill (D9). Assets saved before this change keep only the
  facts they have.
- **Rollback:** migrations are forward-only (ADR-004). An older server refuses
  to start against a database with a migration it does not know, so rolling
  back means restoring the database backup taken before the upgrade.
