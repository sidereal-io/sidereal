<!-- Plain language: Name the actor, condition and observable result. Explain necessary domain terms using the agreed glossary. Keep requirement and scenario headings and WHEN/THEN syntax exactly as required. -->

## MODIFIED Requirements

### Requirement: The image pack declares the format and pixel dimension facets

The image pack SHALL declare three facet schemas, each describing the Asset
version:

- `image.format`, at schema version 2: the label `Format`, a choice whose
  allowed values are `PNG`, `JPEG` and `TIFF`, in that order, and an empty
  unit.
- `image.width`, at schema version 1: the label `Width`, a count, and the
  unit `pixels`.
- `image.height`, at schema version 1: the label `Height`, a count, and the
  unit `pixels`.

#### Scenario: The image pack's schemas
- **WHEN** the image pack registers
- **THEN** the Registry keeps `image.format` with the label `Format`, a choice of `PNG`, `JPEG` or `TIFF`, an empty unit, version 2 and the Asset version as what it describes
- **AND** the Registry keeps `image.width` with the label `Width`, a count, the unit `pixels`, version 1 and the Asset version as what it describes
- **AND** the Registry keeps `image.height` with the label `Height`, a count, the unit `pixels`, version 1 and the Asset version as what it describes


### Requirement: The image pack records an image's format and pixel dimensions

The image pack SHALL register a header Operator, separate from its file-size
Operator, that lists `image.format`, `image.width` and `image.height`. It
SHALL find the format from the version's leading bytes, never from a file
name, and SHALL read the width and height the header stores, ignoring any
EXIF or TIFF orientation. For a TIFF, these are the first image's, whatever
its color type, sample size or compression. It SHALL propose all three
values in one run.

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

#### Scenario: A little-endian TIFF
- **WHEN** core runs the header Operator over a stored little-endian classic TIFF version whose first image is 7,000 by 5,000 pixels
- **THEN** the version's `image.format` is `TIFF`, its `image.width` is 7000 and its `image.height` is 5000

#### Scenario: A big-endian BigTIFF
- **WHEN** core runs the header Operator over a stored big-endian BigTIFF version whose first image is 640 by 480 pixels
- **THEN** the version's `image.format` is `TIFF`, its `image.width` is 640 and its `image.height` is 480

#### Scenario: A TIFF with several images
- **WHEN** core runs the header Operator over a stored TIFF version whose first image is 800 by 600 pixels and whose second image is 200 by 150 pixels
- **THEN** the version's `image.width` is 800 and its `image.height` is 600

#### Scenario: A TIFF the preview cannot decode
- **WHEN** core runs the header Operator over a stored TIFF version whose first image is CMYK, 800 by 600 pixels
- **THEN** the version's `image.format` is `TIFF`, its `image.width` is 800 and its `image.height` is 600

### Requirement: A header that cannot be read fails the header run

When the header Operator cannot read a width and a height, core SHALL store
no value from its run and change no other run. When core can record the
outcome, the run SHALL be `failed` with the reason `the <FORMAT> header
could not be read`, where `<FORMAT>` is `PNG`, `JPEG` or `TIFF` by
signature, or `the image format is not recognized` for none of them. A
header is unreadable when it is cut off, lacks a width or height, gives a
zero one, or is a kind the decoder does not support or refuses.

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
- **WHEN** core runs the header Operator over a stored version whose bytes start with none of the PNG, JPEG and TIFF signatures
- **THEN** its run is `failed` with the reason `the image format is not recognized`

#### Scenario: The file size is still recorded
- **WHEN** core runs the file-size Operator and then the header Operator over a stored version of 1,000 bytes that holds only the start of a JPEG
- **THEN** the file-size run is `succeeded` and the version's `image.file-size` is 1000
- **AND** the header run is `failed`, and the version has no `image.format`, `image.width` or `image.height` value

#### Scenario: A TIFF that ends after its signature
- **WHEN** core runs the header Operator over a stored version that holds only the 4-byte little-endian TIFF signature
- **THEN** its run is `failed` with the reason `the TIFF header could not be read`
- **AND** core stores no `image.format`, `image.width` or `image.height` value for that version

#### Scenario: A TIFF whose first image has no width
- **WHEN** core runs the header Operator over a stored TIFF version whose first image directory has an image length but no image width
- **THEN** its run is `failed` with the reason `the TIFF header could not be read`

#### Scenario: A TIFF whose first image directory lies past its end
- **WHEN** core runs the header Operator over a stored TIFF version whose header points to a first image directory past the end of the file
- **THEN** its run is `failed` with the reason `the TIFF header could not be read`

## ADDED Requirements

### Requirement: A format stored before TIFF support keeps its schema version

A format value stored before this change SHALL keep schema version 1 of
`image.format`, whose allowed values are `PNG` and `JPEG`, and SHALL read as
it did.

#### Scenario: A format stored before this change
- **WHEN** the server starts with this change and reads an Asset saved before it, whose `image.format` is `PNG` under schema version 1
- **THEN** its `facets` hold `image.format` with the value `PNG` and the label `Format`
- **AND** the database keeps both schema versions of `image.format`: version 1 with `PNG` and `JPEG`, and version 2 with `PNG`, `JPEG` and `TIFF`
