<!-- Plain language: Name the actor, condition and observable result. Explain necessary domain terms using the agreed glossary. Keep requirement and scenario headings and WHEN/THEN syntax exactly as required. -->

## ADDED Requirements

### Requirement: The image pack declares the pixel dimension facets

The image pack SHALL declare two facet schemas, each at schema version 1
and each describing the Asset version:

- `image.width`: the label `Width`, a count, and the unit `pixels`.
- `image.height`: the label `Height`, a count, and the unit `pixels`.

#### Scenario: The image pack's schemas
- **WHEN** the image pack registers
- **THEN** the Registry keeps `image.width` with the label `Width`, a count, the unit `pixels`, version 1 and the Asset version as what it describes
- **AND** the Registry keeps `image.height` with the label `Height`, a count, the unit `pixels`, version 1 and the Asset version as what it describes
- **AND** the image pack declares no `image.format` schema

### Requirement: The image pack records an image's pixel dimensions

The image pack SHALL register a header Operator, separate from its file-size
Operator, that lists exactly `image.width` and `image.height`. The header
Operator SHALL find the format from the version's leading bytes, never from
a file name, and read the header by that format. It SHALL read the width and
height that the image's header stores, and SHALL ignore any EXIF
orientation. It SHALL propose both values in one run.

#### Scenario: A PNG
- **WHEN** core runs the header Operator over a stored PNG version whose header gives a width of 4,096 and a height of 2,160 pixels
- **THEN** the version's `image.width` is 4096 and its `image.height` is 2160
- **AND** the provenance of each value names the header Operator and the image pack's version

#### Scenario: A baseline JPEG
- **WHEN** core runs the header Operator over a stored baseline JPEG version whose frame header gives a width of 6,000 and a height of 4,000 pixels
- **THEN** the version's `image.width` is 6000 and its `image.height` is 4000

#### Scenario: A progressive JPEG
- **WHEN** core runs the header Operator over a stored progressive JPEG version whose frame header gives a width of 1,920 and a height of 1,080 pixels
- **THEN** the version's `image.width` is 1920 and its `image.height` is 1080

#### Scenario: A JPEG wider than 16,384 pixels
- **WHEN** core runs the header Operator over a stored baseline JPEG version whose frame header gives a width of 20,000 and a height of 12,000 pixels
- **THEN** the version's `image.width` is 20000 and its `image.height` is 12000

#### Scenario: A JPEG whose metadata comes before its frame header
- **WHEN** core runs the header Operator over a stored JPEG version that holds a 60,000-byte EXIF segment and an ICC profile segment before a frame header that gives 6,000 by 4,000 pixels
- **THEN** the version's `image.width` is 6000 and its `image.height` is 4000

#### Scenario: A JPEG with an EXIF rotation tag
- **WHEN** core runs the header Operator over a stored JPEG version whose frame header gives a width of 6,000 and a height of 4,000 pixels, and whose EXIF orientation says to rotate it 90 degrees
- **THEN** the version's `image.width` is 6000 and its `image.height` is 4000

### Requirement: A header that cannot be read fails the header run

When the header Operator cannot read a width and a height from the bytes,
core SHALL store no value from its run and change no other run. When core
can record the outcome, the run SHALL be `failed` with the reason `the PNG
header could not be read` or `the JPEG header could not be read`, by
signature, or `the image format is not recognized` for neither. A header is
unreadable when it is cut off, gives a zero width or height, is a JPEG kind
the reader does not support, or the decoder refuses it.

#### Scenario: A PNG that ends after its signature
- **WHEN** core runs the header Operator over a stored version that holds only the 8-byte PNG signature
- **THEN** its run is `failed` with the reason `the PNG header could not be read`
- **AND** core stores no `image.width` or `image.height` value for that version

#### Scenario: A PNG whose first chunk is not its header chunk
- **WHEN** core runs the header Operator over a stored version that starts with the PNG signature followed by a chunk other than `IHDR`
- **THEN** its run is `failed` with the reason `the PNG header could not be read`

#### Scenario: A PNG whose header checksum does not match
- **WHEN** core runs the header Operator over a stored version that starts with the PNG signature and a 13-byte `IHDR` chunk whose CRC-32 does not match its type and data
- **THEN** its run is `failed` with the reason `the PNG header could not be read`
- **AND** core stores no `image.width` or `image.height` value for that version

#### Scenario: A PNG with a width of zero
- **WHEN** core runs the header Operator over a stored PNG version whose header gives a width of 0 pixels
- **THEN** its run is `failed` with the reason `the PNG header could not be read`

#### Scenario: A JPEG with no frame header before its image data
- **WHEN** core runs the header Operator over a stored version that starts with the JPEG signature and reaches its start-of-scan marker with no frame header before it
- **THEN** its run is `failed` with the reason `the JPEG header could not be read`
- **AND** core stores no `image.width` or `image.height` value for that version

#### Scenario: A JPEG cut off inside a segment
- **WHEN** core runs the header Operator over a stored version that starts with the JPEG signature and ends partway through a segment before any frame header
- **THEN** its run is `failed` with the reason `the JPEG header could not be read`

#### Scenario: A lossless JPEG
- **WHEN** core runs the header Operator over a stored version that starts with the JPEG signature and has a lossless (`0xC3`) frame header that gives 640 by 480 pixels
- **THEN** its run is `failed` with the reason `the JPEG header could not be read`
- **AND** core stores no `image.width` or `image.height` value for that version

#### Scenario: Bytes of neither format
- **WHEN** core runs the header Operator over a stored version whose bytes start with neither the PNG nor the JPEG signature
- **THEN** its run is `failed` with the reason `the image format is not recognized`

#### Scenario: The file size is still recorded
- **WHEN** core runs the file-size Operator and then the header Operator over a stored version of 1,000 bytes that holds only the start of a JPEG
- **THEN** the file-size run is `succeeded` and the version's `image.file-size` is 1000
- **AND** the header run is `failed`, and the version has no `image.width` or `image.height` value

### Requirement: A failed read of the stored file fails the header run

When reading a stored version's first bytes fails, the header Operator
SHALL fail its run as a storage problem, not as an unknown format. Core
SHALL store no value from the run, and, when it can record the outcome, the
run SHALL be `failed` with the reason `the stored file could not be read`.

#### Scenario: The stored file cannot be read
- **WHEN** core runs the header Operator over a stored version of a PNG and the first read of its stored file fails
- **THEN** its run is `failed` with the reason `the stored file could not be read`
- **AND** core stores no `image.width` or `image.height` value for that version

### Requirement: The header Operator reads at most 8 MiB of a JPEG

The header Operator SHALL read no byte past the first 8 MiB (8,388,608
bytes) of a JPEG. A JPEG whose header, through the end of its first
start-of-scan segment, does not fit there SHALL fail the header run with the
reason `the JPEG header could not be read`.

#### Scenario: A JPEG whose header ends at the limit
- **WHEN** core runs the header Operator over a stored JPEG version whose first start-of-scan segment ends exactly at byte 8,388,608, after a frame header that gives 640 by 480 pixels
- **THEN** its run is `succeeded`, and the version's `image.width` is 640 and its `image.height` is 480

#### Scenario: A JPEG whose header ends one byte past the limit
- **WHEN** core runs the header Operator over a stored JPEG version whose first start-of-scan segment ends at byte 8,388,609, after a frame header that gives 640 by 480 pixels
- **THEN** its run is `failed` with the reason `the JPEG header could not be read`
- **AND** core stores no `image.width` or `image.height` value for that version

#### Scenario: A JPEG whose header runs past 8 MiB
- **WHEN** core runs the header Operator over a stored JPEG version that holds 9 MiB of ICC profile segments before a frame header that gives 640 by 480 pixels
- **THEN** its run is `failed` with the reason `the JPEG header could not be read`
- **AND** core stores no `image.width` or `image.height` value for that version

## MODIFIED Requirements

### Requirement: A run that does not finish cleanly is recorded as failed

When core can record its outcome, a run SHALL be `failed` when the Operator
reports a failure, stops abnormally, or is cancelled, or when core refuses a
proposal. A failed run SHALL record a reason that is not empty, and core
SHALL store no facet value from it. When the database refuses core's
update of a failed outcome, the run SHALL stay `running` with no facet value
stored. Before it records a reason, core SHALL remove every zero character
(U+0000) from it, which PostgreSQL text cannot hold.

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
