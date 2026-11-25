# asset-facets Specification

## Purpose

Let a plugin record checked facts about an Asset version through the public
Operator contract, with core as the only writer of facet values and the
keeper of their provenance and of each operation run.

## Requirements

### Requirement: An Operator runs through the public contract

Core SHALL run an Operator that a pack registered through the public
Registry, over one Asset version. The Operator SHALL reach the version only
through a run-scoped Asset context. Through it, the Operator SHALL be able to
read the version's bytes, propose facet values, write log lines and check
whether the run was cancelled. The context SHALL give the Operator no
database handle, no path in the object store, and no way to set an id, a
hash or provenance.

#### Scenario: A test Operator registered through the public contract
- **WHEN** a test pack registers an Operator that reads the version's first byte through the context and proposes it as the value of a facet it declares, and core runs it over a stored version
- **THEN** core stores that value for that version
- **AND** the stored provenance names the test Operator's id and version

#### Scenario: Log lines go through the host
- **WHEN** an Operator writes the line `looked at the bytes` through its context
- **THEN** the host receives that line, and the Operator wrote it nowhere else

### Requirement: Each facet schema has one owning pack

A pack SHALL declare each facet schema it owns when it registers. A facet
schema SHALL give the facet's name, label, value type, unit, version, and
whether it describes an Asset or an Asset version. A facet's name SHALL
start with its owning pack's id and a dot, as `image.file-size` starts with
`image.`. The Registry SHALL refuse a declaration whose name another
declaration already uses, or whose name does not start with its pack's id
and a dot.

#### Scenario: Two packs declare the same facet
- **WHEN** one test pack has declared the facet `test.size` and a second pack declares `test.size`
- **THEN** the Registry refuses the second declaration with an error that names `test.size`

#### Scenario: A pack declares a facet outside its namespace
- **WHEN** the pack `test` declares a facet named `other.size`
- **THEN** the Registry refuses the declaration with an error that names `other.size`

### Requirement: The Registry refuses a declaration that core cannot store

The Registry SHALL refuse a pack that declares a facet schema or registers an
Operator that core could not record. A facet's name SHALL be two or more
parts of lowercase letters, digits and hyphens, joined by dots. Its version
SHALL be from 1 to 2,147,483,647. Its label SHALL NOT be empty. An Operator's
id and version SHALL NOT be empty. No label, unit, id or version SHALL hold a
zero character (U+0000). The error SHALL name the facet or the Operator.

#### Scenario: A schema version too large to store
- **WHEN** the pack `test` declares `test.size` at version 2147483648
- **THEN** the Registry refuses the pack with an error that names `test.size`
- **AND** the Registry has no schema `test.size`

#### Scenario: A facet name the store does not accept
- **WHEN** the pack `test` declares `test.Size`
- **THEN** the Registry refuses the pack with an error that names `test.Size`

#### Scenario: An Operator version with a zero character
- **WHEN** the pack `test` registers the Operator `test.op` at version `1` U+0000
- **THEN** the Registry refuses the pack with an error that names `test.op`
- **AND** the Registry has no Operator `test.op`

### Requirement: An Operator writes only the facets it declares

An Operator SHALL list the facets it writes when its pack registers it. That
list is the Operator's grants. Until plugin installation adds an approval
step, a pack compiled into the server SHALL have every grant its Operators
list. Core SHALL refuse a proposed value for a facet that is not in the
proposing Operator's list.

#### Scenario: An Operator proposes a facet it did not declare
- **WHEN** an Operator that lists only `test.size` proposes a value for `test.width`, which another schema declares
- **THEN** the run is recorded as failed with a reason that names `test.width`
- **AND** core stores no facet value from that run

### Requirement: An Operator's grants and identity are fixed when it registers

The Registry SHALL record an Operator's id, version and list of facets once,
when its pack registers it. Core SHALL check grants, and record the
producer, from that record only. The Registry SHALL refuse an Operator whose
id another registered Operator already has.

#### Scenario: The Operator's list changes after registration
- **WHEN** an Operator lists only `test.size` when it registers, later answers that it writes `test.size` and `test.width`, and then proposes a value for `test.width`
- **THEN** the run is recorded as failed with a reason that names `test.width`
- **AND** core stores no facet value from that run

#### Scenario: Two Operators with one id
- **WHEN** a pack registers an Operator with the id `test.op`, and a second pack registers another Operator with the id `test.op`
- **THEN** the Registry refuses the second pack with an error that names `test.op`

### Requirement: A refused pack adds nothing to the Registry

When the Registry refuses a pack, it SHALL keep none of the Sources,
Operators, Sinks or facet schemas that pack registered or declared. Every
pack registered before it SHALL stay as it was.

#### Scenario: A pack refused for one of its Operators
- **WHEN** a pack declares the facet `test.good-size`, registers an Operator `test.good` that lists it, and then registers an Operator with the id `test.op`, which an earlier pack's Operator already has
- **THEN** the Registry refuses the pack with an error that names `test.op`
- **AND** the Registry has no Operator `test.good` and no schema `test.good-size`, and running `test.good` fails because no such Operator is registered
- **AND** the earlier pack's Operator `test.op` still runs

### Requirement: Core checks every proposal before it stores any

Core SHALL refuse a proposed value whose facet has no registered schema, or
whose value does not fit the schema's value type. The only value type is a
count: a whole number from 0 to 9,007,199,254,740,991, the largest whole
number a JavaScript client reads exactly. When core refuses any proposal
from a run, it SHALL record the run as failed and SHALL store none of that
run's proposals.

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

#### Scenario: One refused proposal among valid ones
- **WHEN** an Operator proposes a valid value for `test.size` and then a value for a facet it did not declare
- **THEN** the run is recorded as failed
- **AND** core stores neither value

### Requirement: Every run is recorded as an operation run

Core SHALL record one operation run for each time it runs an Operator. The
record SHALL hold its own opaque id, the Operator's id and version, the Asset
version it read, the user who started it, when it started and ended, and
how it ended: `succeeded` or `failed`. A run that ends with no proposals and
no failure SHALL be `succeeded`.

#### Scenario: A successful run
- **WHEN** an administrator's action makes core run an Operator that proposes one valid value
- **THEN** one operation run exists with outcome `succeeded`, the Operator's id and version, the version it read, that administrator as the user who started it, and an end time no earlier than its start time

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

### Requirement: A finished run never changes

Once core has recorded a run as `succeeded` or `failed`, it SHALL NOT change
that record. A `failed` run SHALL have no stored facet values. When core
cannot tell whether a run's values were stored, it SHALL read the run's
record back and report what it holds.

#### Scenario: A failure recorded after the values were stored
- **WHEN** core has stored a run's values and recorded it `succeeded`, and then tries to record that run as failed, as it does after the answer to its commit is lost
- **THEN** the run stays `succeeded` and its values remain
- **AND** core reports the run as succeeded

### Requirement: The Operator reads only the bytes its version recorded

Before it runs an Operator, core SHALL check that the stored object's size
is the size its Asset version recorded. When the sizes differ, core SHALL
record the run as failed with a reason that says the stored object does not
match its version, and SHALL NOT run the Operator.

#### Scenario: The stored object changed on disk
- **WHEN** the stored object of a version of 1,234,567 bytes is cut to 1,000 bytes outside Sidereal, and core runs an Operator over that version
- **THEN** the run is recorded as failed with a reason that says the stored object does not match its version
- **AND** the Operator did not run, and core stores no facet value from that run

### Requirement: Stored facet values carry their provenance

Core SHALL be the only writer of facet values. With each value it stores,
core SHALL record the producing plugin's id and version, the operation run
that produced it, and through that run the user who started it, the facet
schema's version, and when core stored it. Core SHALL take the producer from
the registered Operator that ran, never from anything the Operator proposed.

#### Scenario: Provenance of a stored value
- **WHEN** a run that an administrator started, of a test Operator at version `1.2.3`, succeeds with one value for a version facet at schema version 1
- **THEN** the stored value names the test Operator, version `1.2.3`, that run's id and schema version 1, and has the time it was stored
- **AND** its run names that administrator as the user who started it

### Requirement: A value belongs to what its schema describes

Core SHALL store a value for an Asset version facet against the version the
run read, and a value for an Asset facet against the Asset that owns that
version.

#### Scenario: A value for an Asset version facet
- **WHEN** a run over an Asset version succeeds with one value for a facet whose schema describes the Asset version
- **THEN** the stored value belongs to the Asset version the run read, and to no Asset

#### Scenario: A value for an Asset facet
- **WHEN** a run over an Asset version succeeds with one value for a facet whose schema describes the Asset
- **THEN** the stored value belongs to the Asset that owns that version, and to no Asset version

### Requirement: An Operator's values stay on its own input

The Asset context SHALL give an Operator no way to name the Asset or Asset
version that a proposed value belongs to. Core SHALL decide that from the
run's input alone, and SHALL store no value from a run against any Asset or
Asset version other than the version the run read and the Asset that owns
it. This includes another version of the same Asset.

#### Scenario: Other Assets and versions exist
- **WHEN** the library holds Asset A with versions A1 and A2, and Asset B with version B1, and a run over A1 succeeds with one value for an Asset version facet and one value for an Asset facet
- **THEN** the Asset version value belongs to A1 and the Asset value belongs to A
- **AND** no value from that run belongs to A2, B or B1

### Requirement: An Operator run makes no Asset

Until core accepts derived Assets from an Operator, core SHALL record an
Operator run that proposes an Asset as failed. Core
SHALL add no Asset and no Asset version for that run, and SHALL store no
facet value from it. When the Operator reports success after proposing an
Asset, the reason SHALL say that an Operator run proposed an Asset.

#### Scenario: An Operator proposes an Asset and reports success
- **WHEN** an Operator proposes a valid value for `test.size`, then proposes an Asset of kind `image`, ignores the refusal and reports success
- **THEN** its operation run is `failed` with a reason that says an operator run proposed an asset
- **AND** the library holds the same Assets and Asset versions as before the run
- **AND** core stores no facet value from that run

#### Scenario: An Operator proposes an Asset and stops at the refusal
- **WHEN** an Operator proposes a valid value for `test.size`, then proposes an Asset of kind `image`, and returns the failure `could not make a preview` when the context refuses it
- **THEN** its operation run is `failed` with the reason `could not make a preview`
- **AND** the library holds the same Assets and Asset versions as before the run
- **AND** core stores no facet value from that run

### Requirement: A stored value keeps the schema it was written under

Core SHALL keep each facet schema version that a stored value was written
under, unchanged. A value SHALL be read with the label and unit of its own
schema version. When a pack declares a schema version that core already
keeps, with a different label, unit, value type, scope or owner, core SHALL
record a run that writes that facet as failed, with a reason that names the
facet and its version, and SHALL store none of its values.

#### Scenario: A facet changed without a new version
- **WHEN** a value of `test.size` at schema version 1 with the unit `bytes` is stored, and a later build declares `test.size` version 1 with the unit `KiB` and runs an Operator that writes it
- **THEN** that run is recorded as failed with a reason that names `test.size` and version 1
- **AND** the stored value still reads with the unit `bytes`

### Requirement: The image pack records an image's file size

The image pack SHALL declare the facet schema `image.file-size`, with the
label `File size`, a whole number of zero or more as its value type, the unit
`bytes`, schema version 1, and the Asset version as what it describes. The
image pack SHALL register an Operator that lists `image.file-size` and
proposes, as its value, the number of bytes in the version it reads.

#### Scenario: A PNG of known size
- **WHEN** core runs the file-size Operator over a stored PNG version of 1,234,567 bytes
- **THEN** the version's `image.file-size` value is 1234567
- **AND** its provenance names the file-size Operator and the image pack's version

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
