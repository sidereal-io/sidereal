<!-- Plain language: Name the actor, condition and observable result. Explain necessary domain terms using the agreed glossary. Keep requirement and scenario headings and WHEN/THEN syntax exactly as required. -->

## ADDED Requirements

### Requirement: A choice facet declares its allowed values

A facet schema whose value type is a choice SHALL list its allowed values:
one or more pieces of text. Each allowed value SHALL NOT be empty, SHALL NOT
hold a zero character (U+0000), and SHALL differ from every other allowed
value of that schema. The Registry SHALL refuse a pack that declares a choice
facet that breaks any of these rules, with an error that names the facet,
and SHALL keep nothing that pack registered or declared.

#### Scenario: A choice with no allowed values
- **WHEN** the pack `test` declares `test.kind` as a choice with no allowed values
- **THEN** the Registry refuses the pack with an error that names `test.kind`
- **AND** the Registry has no schema `test.kind`

#### Scenario: A choice with an empty allowed value
- **WHEN** the pack `test` declares `test.kind` as a choice whose allowed values are `a` and the empty text
- **THEN** the Registry refuses the pack with an error that names `test.kind`

#### Scenario: A choice that allows one value twice
- **WHEN** the pack `test` declares `test.kind` as a choice whose allowed values are `a`, `b` and `a`
- **THEN** the Registry refuses the pack with an error that names `test.kind`

#### Scenario: An allowed value with a zero character
- **WHEN** the pack `test` declares `test.kind` as a choice whose allowed values are `a` and `b` U+0000
- **THEN** the Registry refuses the pack with an error that names `test.kind`

#### Scenario: A well-formed choice
- **WHEN** the pack `test` declares `test.kind` as a choice whose allowed values are `a` and `b`
- **THEN** the Registry keeps the schema `test.kind`, with the allowed values `a` and `b`

### Requirement: The image pack declares the format and pixel dimension facets

The image pack SHALL declare three facet schemas, each at schema version 1
and each describing the Asset version:

- `image.format`: the label `Format`, a choice whose allowed values are `PNG`
  and `JPEG`, and an empty unit.
- `image.width`: the label `Width`, a count, and the unit `pixels`.
- `image.height`: the label `Height`, a count, and the unit `pixels`.

#### Scenario: The image pack's schemas
- **WHEN** the image pack registers
- **THEN** the Registry keeps `image.format` with the label `Format`, a choice of `PNG` or `JPEG`, an empty unit, version 1 and the Asset version as what it describes
- **AND** the Registry keeps `image.width` with the label `Width`, a count, the unit `pixels`, version 1 and the Asset version as what it describes
- **AND** the Registry keeps `image.height` with the label `Height`, a count, the unit `pixels`, version 1 and the Asset version as what it describes

### Requirement: The image pack records an image's format and pixel dimensions

The image pack SHALL register a header Operator, separate from its file-size
Operator, that lists `image.format`, `image.width` and `image.height`. The
header Operator SHALL find the format from the version's leading bytes,
never from a file name. It SHALL read the width and height that the image's
header stores, and SHALL ignore any EXIF orientation. It SHALL propose all
three values in one run.

#### Scenario: A PNG
- **WHEN** core runs the header Operator over a stored PNG version whose header gives a width of 4,096 and a height of 2,160 pixels
- **THEN** the version's `image.format` is `PNG`, its `image.width` is 4096 and its `image.height` is 2160
- **AND** the provenance of each value names the header Operator and the image pack's version

#### Scenario: A baseline JPEG
- **WHEN** core runs the header Operator over a stored baseline JPEG version whose frame header gives a width of 6,000 and a height of 4,000 pixels
- **THEN** the version's `image.format` is `JPEG`, its `image.width` is 6000 and its `image.height` is 4000

#### Scenario: A progressive JPEG
- **WHEN** core runs the header Operator over a stored progressive JPEG version whose frame header gives a width of 1,920 and a height of 1,080 pixels
- **THEN** the version's `image.format` is `JPEG`, its `image.width` is 1920 and its `image.height` is 1080

#### Scenario: A JPEG wider than 16,384 pixels
- **WHEN** core runs the header Operator over a stored baseline JPEG version whose frame header gives a width of 20,000 and a height of 12,000 pixels
- **THEN** the version's `image.format` is `JPEG`, its `image.width` is 20000 and its `image.height` is 12000

#### Scenario: A JPEG whose metadata comes before its frame header
- **WHEN** core runs the header Operator over a stored JPEG version that holds a 60,000-byte EXIF segment and an ICC profile segment before a frame header that gives 6,000 by 4,000 pixels
- **THEN** the version's `image.width` is 6000 and its `image.height` is 4000

#### Scenario: A JPEG with an EXIF rotation tag
- **WHEN** core runs the header Operator over a stored JPEG version whose frame header gives a width of 6,000 and a height of 4,000 pixels, and whose EXIF orientation says to rotate it 90 degrees
- **THEN** the version's `image.width` is 6000 and its `image.height` is 4000

### Requirement: A header that cannot be read fails the header run

When the header Operator cannot read a width and a height from the bytes,
its run SHALL fail, store no value and change no other run. The reason SHALL
be `the PNG header could not be read` or `the JPEG header could not be read`,
by signature, or `the image format is not recognized` for neither. A header
is unreadable when it is cut off, malformed or damaged, gives a zero width
or height, or is a JPEG kind the reader does not support.

#### Scenario: A PNG that ends after its signature
- **WHEN** core runs the header Operator over a stored version that holds only the 8-byte PNG signature
- **THEN** its run is `failed` with the reason `the PNG header could not be read`
- **AND** core stores no `image.format`, `image.width` or `image.height` value for that version

#### Scenario: A PNG whose first chunk is not its header chunk
- **WHEN** core runs the header Operator over a stored version that starts with the PNG signature followed by a chunk other than `IHDR`
- **THEN** its run is `failed` with the reason `the PNG header could not be read`

#### Scenario: A PNG whose header checksum does not match
- **WHEN** core runs the header Operator over a stored version that starts with the PNG signature and a 13-byte `IHDR` chunk whose CRC-32 does not match its type and data
- **THEN** its run is `failed` with the reason `the PNG header could not be read`
- **AND** core stores no `image.format`, `image.width` or `image.height` value for that version

#### Scenario: A PNG with a width of zero
- **WHEN** core runs the header Operator over a stored PNG version whose header gives a width of 0 pixels
- **THEN** its run is `failed` with the reason `the PNG header could not be read`


#### Scenario: A JPEG with no frame header before its image data
- **WHEN** core runs the header Operator over a stored version that starts with the JPEG signature and reaches its start-of-scan marker with no frame header before it
- **THEN** its run is `failed` with the reason `the JPEG header could not be read`
- **AND** core stores no `image.format`, `image.width` or `image.height` value for that version

#### Scenario: A JPEG cut off inside a segment
- **WHEN** core runs the header Operator over a stored version that starts with the JPEG signature and ends partway through a segment before any frame header
- **THEN** its run is `failed` with the reason `the JPEG header could not be read`

#### Scenario: A lossless JPEG
- **WHEN** core runs the header Operator over a stored version that starts with the JPEG signature and has a lossless (`0xC3`) frame header that gives 640 by 480 pixels
- **THEN** its run is `failed` with the reason `the JPEG header could not be read`
- **AND** core stores no `image.format`, `image.width` or `image.height` value for that version

#### Scenario: Bytes of neither format
- **WHEN** core runs the header Operator over a stored version whose bytes start with neither the PNG nor the JPEG signature
- **THEN** its run is `failed` with the reason `the image format is not recognized`

#### Scenario: The file size is still recorded
- **WHEN** core runs the file-size Operator and then the header Operator over a stored version of 1,000 bytes that holds only the start of a JPEG
- **THEN** the file-size run is `succeeded` and the version's `image.file-size` is 1000
- **AND** the header run is `failed`, and the version has no `image.format`, `image.width` or `image.height` value

### Requirement: The header Operator reads at most 32 MiB of a JPEG

The header Operator SHALL read no byte past the first 32 MiB (33,554,432
bytes) of a JPEG. A JPEG whose header, up to its start-of-scan marker, does
not fit there SHALL fail the header run with the reason `the JPEG header
could not be read`.

#### Scenario: A JPEG whose header runs past 32 MiB
- **WHEN** core runs the header Operator over a stored JPEG version that holds 33 MiB of ICC profile segments before a frame header that gives 640 by 480 pixels
- **THEN** its run is `failed` with the reason `the JPEG header could not be read`
- **AND** core stores no `image.format`, `image.width` or `image.height` value for that version

## MODIFIED Requirements

### Requirement: Core checks every proposal before it stores any

Core SHALL refuse a proposed value whose facet has no registered schema, or
whose value does not fit the schema's value type. There are two value types.
A count is a whole number from 0 to 9,007,199,254,740,991, the largest whole
number a JavaScript client reads exactly. A choice is a piece of text equal
to one of the allowed values its schema lists, letter case included. When
core refuses any proposal from a run, it SHALL record the run as failed and
SHALL store none of that run's proposals.

#### Scenario: An unknown facet
- **WHEN** an Operator that lists `test.missing` proposes a value for it, and no pack declares a schema named `test.missing`
- **THEN** the run is recorded as failed with a reason that names `test.missing`
- **AND** core stores no facet value from that run

#### Scenario: A value that does not fit its type
- **WHEN** an Operator proposes the value -1 for a count facet
- **THEN** the run is recorded as failed with a reason that names the facet
- **AND** core stores no facet value from that run

#### Scenario: A count above the largest count
- **WHEN** an Operator proposes the value 9007199254740992 for a count facet
- **THEN** the run is recorded as failed with a reason that names the facet
- **AND** core stores no facet value from that run

#### Scenario: The largest count
- **WHEN** an Operator proposes the value 9007199254740991 for a count facet
- **THEN** core stores the value 9007199254740991

#### Scenario: An allowed choice
- **WHEN** an Operator proposes the text `PNG` for a choice facet whose allowed values are `PNG` and `JPEG`
- **THEN** core stores the text `PNG`

#### Scenario: A choice the schema does not allow
- **WHEN** an Operator proposes the text `GIF`, or the text `png`, for a choice facet whose allowed values are `PNG` and `JPEG`
- **THEN** the run is recorded as failed with a reason that names the facet
- **AND** core stores no facet value from that run

#### Scenario: A value of the other type
- **WHEN** an Operator proposes the text `12` for a count facet, or the number 1 for a choice facet
- **THEN** the run is recorded as failed with a reason that names the facet
- **AND** core stores no facet value from that run

#### Scenario: One refused proposal among valid ones
- **WHEN** an Operator proposes a valid value for `test.size` and then a value for a facet it did not declare
- **THEN** the run is recorded as failed
- **AND** core stores neither value

### Requirement: A run that does not finish cleanly is recorded as failed

When core can record its outcome, a run SHALL be `failed` when the Operator
reports a failure, stops abnormally, or is cancelled, or when core refuses a
proposal. A failed run SHALL record a reason that is not empty, and core
SHALL store no facet value from it. When core cannot record the outcome, the
run's record SHALL stay as it was, with no facet value stored. Before it records a reason, core SHALL remove every zero character
(U+0000) from it, because PostgreSQL text cannot hold one.

#### Scenario: The Operator reports a failure
- **WHEN** an Operator returns the failure `could not read the header`
- **THEN** its operation run is `failed` with the reason `could not read the header`
- **AND** core stores no facet value from that run

#### Scenario: The Operator reports a failure with no reason
- **WHEN** an Operator returns a failure whose reason is empty
- **THEN** its operation run is `failed` with the reason `the operator gave no reason`

#### Scenario: A failure reason holds a zero character
- **WHEN** an Operator returns the failure `bad` U+0000 `header`, or a failure whose reason is only U+0000
- **THEN** its operation run is `failed` with the reason `badheader`, or `the operator gave no reason`
- **AND** core stores no facet value from that run

#### Scenario: A proposed facet name holds a zero character
- **WHEN** an Operator proposes a value for the facet `test.` U+0000 `x`, which no schema declares
- **THEN** its operation run is `failed` with a reason that names `test.x`

#### Scenario: The Operator stops abnormally
- **WHEN** an Operator panics during its run
- **THEN** its operation run is `failed` with a reason that says the Operator stopped
- **AND** core stores no facet value from that run

#### Scenario: The host cancels the run
- **WHEN** the host cancels a run after the Operator proposed a valid value
- **THEN** the operation run is `failed` with a reason that says it was cancelled
- **AND** core stores no facet value from that run

#### Scenario: The outcome of a failed run cannot be recorded
- **WHEN** an Operator returns the failure `could not read the header`, and the database refuses core's update of the run's outcome
- **THEN** the run's record stays `running`
- **AND** core stores no facet value from that run

### Requirement: A stored value keeps the schema it was written under

Core SHALL keep each facet schema version that a stored value was written
under, unchanged, with a choice's allowed values. A value SHALL be read with
the label and unit of its own schema version. When a pack declares a kept
schema version with a different label, unit, value type, scope, owner, or
allowed values or their order, core SHALL record a run that writes that
facet as failed, with a reason that names the facet and its version, and
SHALL store none of its values.

#### Scenario: A facet changed without a new version
- **WHEN** a value of `test.size` at schema version 1 with the unit `bytes` is stored, and a later build declares `test.size` version 1 with the unit `KiB` and runs an Operator that writes it
- **THEN** that run is recorded as failed with a reason that names `test.size` and version 1
- **AND** the stored value still reads with the unit `bytes`

#### Scenario: Allowed values changed without a new version
- **WHEN** a value of `test.kind` at schema version 1, a choice of `a` or `b`, is stored, and a later build declares `test.kind` version 1 as a choice of `a`, `b` or `c` and runs an Operator that writes it
- **THEN** that run is recorded as failed with a reason that names `test.kind` and version 1
- **AND** core stores no value from that run

#### Scenario: A new version adds an allowed value
- **WHEN** a value of `test.kind` at schema version 1, a choice of `a` or `b`, is stored, and a later build declares `test.kind` version 2 as a choice of `a`, `b` or `c` and runs an Operator that proposes `c`
- **THEN** core stores `c` under schema version 2
- **AND** the value stored under version 1 still reads as before
