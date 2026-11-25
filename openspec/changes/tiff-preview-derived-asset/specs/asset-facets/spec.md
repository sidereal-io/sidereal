<!-- Plain language: Name the actor, condition and observable result. Explain necessary domain terms using the agreed glossary. Keep requirement and scenario headings and WHEN/THEN syntax exactly as required. -->

## RENAMED Requirements

- FROM: `### Requirement: An Operator run makes no Asset`
- TO: `### Requirement: An Operator run proposes no Asset as a Source does`

## MODIFIED Requirements

### Requirement: An Operator's grants and identity are fixed when it registers

The Registry SHALL record an Operator's id, version, list of facets and list
of accepted media types once, when its pack registers it. Core SHALL check
grants, decide which versions the Operator runs on, and record the producer,
from that record only. The Registry SHALL refuse an Operator whose id
another registered Operator already has.

#### Scenario: The Operator's list changes after registration
- **WHEN** an Operator lists only `test.size` when it registers, later answers that it writes `test.size` and `test.width`, and then proposes a value for `test.width`
- **THEN** the run is recorded as failed with a reason that names `test.width`
- **AND** core stores no facet value from that run

#### Scenario: The accepted media types change after registration
- **WHEN** an Operator accepts only `image/png` when it registers, later answers that it accepts `image/png` and `image/gif`, and core is asked to run it over an original's version whose media type is `image/gif`
- **THEN** core records no run for that version

#### Scenario: Two Operators with one id
- **WHEN** a pack registers an Operator with the id `test.op`, and a second pack registers another Operator with the id `test.op`
- **THEN** the Registry refuses the second pack with an error that names `test.op`

### Requirement: An Operator run proposes no Asset as a Source does

An Operator SHALL make a new Asset only by proposing a preview, as the
derived-assets capability says. Core SHALL record an Operator run that
proposes an Asset as a Source does as failed. Core SHALL add no Asset and
no Asset version for that run, and SHALL store no facet value from it. When
the Operator reports success after proposing an Asset, the reason SHALL say
that an Operator run proposed an Asset.

#### Scenario: An Operator proposes an Asset and reports success
- **WHEN** an Operator proposes a valid value for `test.size`, then proposes an Asset of kind `image` as a Source does, ignores the refusal and reports success
- **THEN** its operation run is `failed` with a reason that says an operator run proposed an asset
- **AND** the library holds the same Assets and Asset versions as before the run
- **AND** core stores no facet value from that run

#### Scenario: An Operator proposes an Asset and stops at the refusal
- **WHEN** an Operator proposes a valid value for `test.size`, then proposes an Asset of kind `image` as a Source does, and returns the failure `could not make a preview` when the context refuses it
- **THEN** its operation run is `failed` with the reason `could not make a preview`
- **AND** the library holds the same Assets and Asset versions as before the run
- **AND** core stores no facet value from that run

### Requirement: The image pack records an image's pixel dimensions

The image pack SHALL register a header Operator, separate from its file-size
Operator, that lists exactly `image.width` and `image.height`. The header
Operator SHALL read the header by the version's media type, which the
Source set from the file's bytes, never by a file name. It SHALL read the
width and height that the image's header stores, and SHALL ignore any EXIF
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
can record the outcome, the run SHALL be `failed` with the reason
`the <FORMAT> header could not be read`, where `<FORMAT>` is `PNG`, `JPEG`
or `TIFF` by the version's media type. A header is unreadable when it is
cut off, lacks a width or height, gives a zero one, is a kind the reader
does not support, or the decoder refuses it.

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
- **WHEN** core is asked to run the header Operator over an original's version whose media type is `image/gif`
- **THEN** core records no header run for that version, and stores no `image.width` or `image.height` value for it

#### Scenario: The file size is still recorded
- **WHEN** core runs the file-size Operator and then the header Operator over a stored version of 1,000 bytes that holds only the start of a JPEG
- **THEN** the file-size run is `succeeded` and the version's `image.file-size` is 1000
- **AND** the header run is `failed`, and the version has no `image.width` or `image.height` value

### Requirement: A failed read of the stored file fails the header run

When any read of a stored version's bytes fails, the header Operator SHALL
fail its run as a storage problem, not as an unreadable header. Core SHALL
store no value from the run, and, when it can record the outcome, the run
SHALL be `failed` with the reason `the stored file could not be read`.

#### Scenario: The stored file cannot be read
- **WHEN** core runs the header Operator over a stored version of a PNG and the first read of its stored file fails
- **THEN** its run is `failed` with the reason `the stored file could not be read`
- **AND** core stores no `image.width` or `image.height` value for that version

#### Scenario: A later read fails
- **WHEN** core runs the header Operator over a stored version of a JPEG whose frame header lies after two valid EXIF segments totaling more than 100,000 bytes, and the second read of its stored file fails
- **THEN** its run is `failed` with the reason `the stored file could not be read`

## ADDED Requirements

### Requirement: The header Operator reads a TIFF's first image

The header Operator SHALL read a TIFF's width and height from its first
image, whatever that image's color type, sample size or compression, and
SHALL ignore any later image and any TIFF orientation. Preview eligibility and
metadata unrelated to dimensions SHALL NOT affect this read.

#### Scenario: A little-endian TIFF
- **WHEN** core runs the header Operator over a stored little-endian classic TIFF version whose first image is 7,000 by 5,000 pixels
- **THEN** the version's `image.width` is 7000 and its `image.height` is 5000

#### Scenario: A big-endian BigTIFF
- **WHEN** core runs the header Operator over a stored big-endian BigTIFF version whose first image is 640 by 480 pixels
- **THEN** the version's `image.width` is 640 and its `image.height` is 480

#### Scenario: A TIFF with several images
- **WHEN** core runs the header Operator over a stored TIFF version whose first image is 800 by 600 pixels and whose second image is 200 by 150 pixels
- **THEN** the version's `image.width` is 800 and its `image.height` is 600

#### Scenario: A TIFF the preview cannot decode
- **WHEN** core runs the header Operator over a stored TIFF version whose first image is CMYK, 800 by 600 pixels
- **THEN** the version's `image.width` is 800 and its `image.height` is 600

#### Scenario: A TIFF has a sample array the preview cannot use
- **WHEN** core runs the header Operator over an 800 by 600 TIFF whose first image has more than 64 samples per pixel
- **THEN** the version's `image.width` is 800 and its `image.height` is 600

### Requirement: A TIFF header that cannot be read fails the header run

When the header Operator cannot read a TIFF's first image directory, or the
directory has no width or length, the run SHALL fail as "A header that
cannot be read fails the header run" says, with the reason
`the TIFF header could not be read`.

#### Scenario: A TIFF that ends after its signature
- **WHEN** core runs the header Operator over a stored version that holds only the 4-byte little-endian TIFF signature
- **THEN** its run is `failed` with the reason `the TIFF header could not be read`
- **AND** core stores no `image.width` or `image.height` value for that version

#### Scenario: A TIFF whose first image has no width
- **WHEN** core runs the header Operator over a stored TIFF version whose first image directory has an image length but no image width
- **THEN** its run is `failed` with the reason `the TIFF header could not be read`

#### Scenario: A TIFF whose first image directory lies past its end
- **WHEN** core runs the header Operator over a stored TIFF version whose header points to a first image directory past the end of the file
- **THEN** its run is `failed` with the reason `the TIFF header could not be read`

### Requirement: The media type decides how the header is read

The header Operator SHALL read the bytes as the format its version's media
type names, and SHALL NOT look for another format's signature. Bytes that
do not match the media type SHALL fail as that format's header.

#### Scenario: Bytes that do not match the media type
- **WHEN** core runs the header Operator over a stored version whose media type is `image/png` and whose bytes are a JPEG of 640 by 480 pixels
- **THEN** its run is `failed` with the reason `the PNG header could not be read`
- **AND** core stores no `image.width` or `image.height` value for that version

### Requirement: The Asset context gives an Operator its version's media type

An Operator's Asset context SHALL give the media type core recorded for the
version it reads. It SHALL give the Operator no way to change it. A
Source's context SHALL give no media type, because the Source is what
recognizes its candidate.

#### Scenario: The Operator reads the version's media type
- **WHEN** core runs a test Operator that accepts `image/png` over a stored version whose media type is `image/png`, and the Operator proposes the length of the media type it reads through the context as a facet value
- **THEN** core stores the value 9 for that version

#### Scenario: A Source sees no media type
- **WHEN** a test Source reads the media type through its context during its run
- **THEN** the context gives it none

### Requirement: An Operator declares the media types it accepts

An Operator SHALL declare, when it registers, the media types of
originals' versions it accepts, such as `image/png`. The Registry SHALL
refuse a pack with an Operator whose list is empty, or holds a media type
that is not one lowercase `type/subtype` with no parameters, with an error
that names the Operator.

#### Scenario: An Operator that accepts PNG and JPEG
- **WHEN** a test pack registers an Operator that accepts `image/png` and `image/jpeg`
- **THEN** the Registry records that this Operator accepts exactly `image/png` and `image/jpeg`

#### Scenario: An Operator that accepts nothing
- **WHEN** a test pack registers an Operator whose list of accepted media types is empty
- **THEN** the Registry refuses the pack with an error that names the Operator, and adds nothing from it

#### Scenario: A malformed media type
- **WHEN** a test pack registers an Operator that accepts `Image/PNG; q=1`
- **THEN** the Registry refuses the pack with an error that names the Operator, and adds nothing from it

#### Scenario: The image pack's Operators
- **WHEN** the image pack registers
- **THEN** its file-size, header and preview Operators each accept exactly `image/png`, `image/jpeg` and `image/tiff`

### Requirement: Core runs an Operator only on an original's version it accepts

Core SHALL run an Operator over an Asset version only when the version
belongs to an original and its media type is one the Operator accepts. For
any other version, core SHALL record no operation run, store no facet value
and make no Asset, and SHALL report that the Operator does not accept the
version. A derived Asset's version, such as a preview, SHALL be accepted by
no Operator. The Registry SHALL also refuse to run an Operator over a media
type it does not accept.

#### Scenario: A media type the Operator does not accept
- **WHEN** core is asked to run a test Operator that accepts `image/png` over an original's version whose media type is `image/jpeg`
- **THEN** core records no operation run for that version, and reports that the Operator does not accept it
- **AND** the Operator did not run

#### Scenario: A preview's version
- **WHEN** core is asked to run a test Operator that accepts `image/jpeg` over the version of a preview, whose media type is `image/jpeg`
- **THEN** core records no operation run for that version, and the preview has no facet value and no preview

#### Scenario: The Registry refuses a media type the Operator does not accept
- **WHEN** a host runs a test Operator that accepts `image/png` through the Registry with the media type `image/gif`
- **THEN** the Registry reports that the Operator does not accept it, and the Operator did not run
