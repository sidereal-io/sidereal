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
  `image.measure-file-size`. The pack depends on `plugin-abi` only, and
  `server/scripts/check-arch.sh` enforces that.
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
  `facet_schemas`, the image pack, the upload route's Operator list and log
  lines, the asset JSON, and the asset page. No authorization or trust
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
is `NULL` exactly when `value_type` is not `'choice'`, so existing rows stay
valid. `type_text` returns `"choice"` for a choice. `store` writes the
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

### 5. A hand-written header reader in the image pack

The header Operator, `image.read-header`, reads the first 8 bytes and picks
the format from the signature, as the Source does. Both share one signature
function.

- **PNG:** read the 25 bytes after the signature. The first chunk must have
  length 13 and type `IHDR`. Width and height are the next two big-endian
  32-bit numbers. Each must be from 1 to 2,147,483,647, the PNG limit.
- **JPEG:** walk the markers from offset 2. Skip any `0xFF` fill bytes before
  a marker. A marker with no length (`0x01`, `0xD0` to `0xD7`) is skipped. For
  any other marker, read its 2-byte length and jump past the segment without
  reading its body. A start-of-frame marker (`0xC0` to `0xCF`, except `0xC4`,
  `0xC8` and `0xCC`) gives height then width as big-endian 16-bit numbers
  after a 1-byte precision. Reaching start-of-scan (`0xDA`), end-of-image
  (`0xD9`), the end of the bytes, a byte that is not a marker, or a length
  below 2 is a failure. A width or height of zero is a failure, including the
  JPEG case where a later marker defines the height.
- Each failure returns `OperatorError::Failed` with the reason the spec
  gives. The Operator then proposes `image.format`, `image.width` and
  `image.height`.

The walk reads 4 bytes per segment and never the segment bodies, so a large
EXIF or ICC segment costs one read. It checks `is_cancelled` once per
segment, and stops at the end of the bytes, so a crafted file cannot loop.

- *Alternative:* the `imagesize` crate. It reads from a slice or a file, so
  the Operator would first copy bytes out of the context, up to an unknown
  depth for JPEG. It also adds a dependency to a pack that has none, and the
  two formats need about 80 lines.

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
- [A JPEG whose start-of-frame comes after a very long run of segments makes
  many small reads] → Each read is one positioned read of 4 bytes, the walk
  ends at the end of the bytes, and an upload is at most 100 MiB. Cancellation
  is checked per segment.
- [Some valid files fail the header run: a JPEG whose height is set by a later
  marker, or an Apple-optimized PNG whose first chunk is `CgBI`] → The run
  fails with a clear reason and the file size is still recorded. These files
  are rare in astrophotography output, and #527 shows processing failures.
- [Adding `PNG` or `JPEG` as display text makes it the stored value too, so
  renaming it later needs a new schema version] → That is the cost D3
  accepted. The values are the formats' common names and are not expected to
  change.
- [The upload answers later, because it now waits for two runs] → The header
  run reads a few hundred bytes at most for these formats; the delay is small
  next to the upload itself.

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
