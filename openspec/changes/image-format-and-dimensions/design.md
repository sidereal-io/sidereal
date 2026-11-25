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
  `facet_schemas`, the image pack and its first crates.io dependency
  (`image`), the upload route's Operator list and log lines, the asset JSON,
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

### 5. The image pack reads headers with the `image` crate

The header Operator, `image.read-header`, reads the first 8 bytes and picks
the format from the signature, as the Source does. Both share one signature
function. Bytes that start with neither signature fail with `the image format
is not recognized`.

The Operator does not parse PNG or JPEG itself. It uses the pure-Rust
[`image`](https://crates.io/crates/image) crate (0.25), with default features
off and only `png` and `jpeg` on:
`ImageReader::with_format(reader, format).into_dimensions()`. It names the
format from the signature, so the crate never guesses from a file name.
`into_dimensions` builds the format's decoder, which reads the header, and
returns the width and height the header stores. It decodes no pixel data and
applies no EXIF orientation (D8).

- **The reader.** `into_dimensions` needs `BufRead + Seek`. A small adapter
  in the pack, `CandidateReader`, wraps the `AssetContext`. It keeps a
  64 KiB buffer (`AssetContext::MAX_READ`) and fills it with one
  `read_candidate` call at the current offset. `seek` moves the offset and
  empties the buffer only when the new offset lies outside it. A short read
  is not a failure: the adapter returns what it got, and only a read of 0
  bytes means the end of the candidate. Before each fill it checks
  `is_cancelled`, and returns an I/O error when the run is cancelled.
- **The bound on reads.** Each fill reads up to 64 KiB past the last, so a
  run over a candidate of n bytes makes at most n / 65,536 + c
  `read_candidate` calls, where c is a small constant for seeks back to
  bytes already passed. For the 100 MiB upload limit that is about 1,600
  reads. No segment count or byte cap applies, so no valid file is refused
  for its layout.
- **PNG.** The crate's PNG decoder reads the chunks up to the first image
  data chunk. It checks each chunk's CRC-32, including `IHDR`'s, so a
  damaged header fails rather than giving wrong dimensions. A first chunk
  that is not a 13-byte `IHDR`, or a header cut off early, also fails.
- **JPEG.** The crate's JPEG decoder (zune-jpeg) parses the markers up to
  the frame header. In `image` 0.25, `JpegDecoder::new` first reads the
  whole candidate into memory with `read_to_end`, through the adapter's
  64 KiB fills. The reads stay within the bound above. The memory cost is
  in Risks.
- **Failures.** Any error from the crate, including one from the adapter,
  fails the run. The reason is `the PNG header could not be read` or `the
  JPEG header could not be read`, by signature. The Operator also fails a
  width or height of 0, and a PNG width or height above 2,147,483,647, the
  PNG limit, whatever the crate accepts. The Operator then proposes
  `image.format`, `image.width` and `image.height`.

- *Alternative:* a hand-written PNG and JPEG header reader, about 80 lines.
  It adds no dependency, but the review found it needs its own bound on
  work, its own CRC-32 and its own structural checks, each a place to get
  wrong. The `image` crate is used widely, and #526 needs it to render TIFF
  previews.
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
  segments or fill bytes] → The adapter reads in 64 KiB fills, so a run
  costs at most size / 65,536 + c reads, about 1,600 for 100 MiB, whatever
  the layout. Task 3.3 tests the bound with such a file.
- [The `image` 0.25 JPEG decoder reads the whole file into memory before it
  reads the header, so one JPEG header run holds up to 100 MiB, the upload
  limit, until it ends] → The run is short and runs once per upload, after
  the Asset is saved. Each upload in flight adds at most one such buffer.
  If the crate later reads JPEG headers without the whole file, the bound on
  reads still holds and the memory cost falls.
- [Some valid files fail the header run: a JPEG whose height is set by a
  later marker, a JPEG type the decoder does not support (such as lossless or
  arithmetic-coded), or an Apple-optimized PNG whose first chunk is `CgBI`]
  → The run fails with a clear reason and the file size is still recorded.
  These files are rare in astrophotography output, and #527 shows
  processing failures.
- [A new `image` release changes which files it accepts] → `Cargo.lock`
  pins the version, the header tests cover each case the specs name, and
  Dependabot updates arrive as pull requests that run those tests.
- [Adding `PNG` or `JPEG` as display text makes it the stored value too, so
  renaming it later needs a new schema version] → That is the cost D3
  accepted. The values are the formats' common names and are not expected to
  change.
- [The upload answers later, because it now waits for two runs] → A PNG
  header run reads one 64 KiB fill in the usual case. A JPEG header run
  reads the file once from local storage, which is small next to receiving
  it over the network.

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
  back means restoring the database backup taken before the upgrade. Until a
  choice value is stored, dropping the column by hand and removing the
  migration's row from `_sqlx_migrations` would also work, but it is not a
  supported path.
