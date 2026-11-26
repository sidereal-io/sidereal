<!-- Plain language: Name the actor, condition and observable result. Explain necessary domain terms using the agreed glossary. Keep requirement and scenario headings and WHEN/THEN syntax exactly as required. -->

## Purpose

Let the image pack make a browser-ready preview of a PNG, JPEG or TIFF
version, so the library can show an image the browser cannot show itself.

## ADDED Requirements

### Requirement: The image pack registers a preview Operator

The image pack SHALL register an Operator with the id `image.make-preview`,
separate from its file-size and header Operators. It SHALL declare that it
makes previews, SHALL list no facets, and SHALL accept the media types
`image/png`, `image/jpeg` and `image/tiff`. Each successful run SHALL
propose one preview of kind `image` and media type `image/jpeg`.

#### Scenario: The preview Operator's registration
- **WHEN** the image pack registers
- **THEN** the Registry holds the Operator `image.make-preview`, which makes previews, lists no facets and accepts exactly `image/png`, `image/jpeg` and `image/tiff`

#### Scenario: A PNG's preview
- **WHEN** core runs the preview Operator over a stored 8-bit RGB PNG version of `nebula.png`
- **THEN** its run is `succeeded`, and the version's preview is a derived Asset of kind `image` whose version has the media type `image/jpeg`

### Requirement: A preview is an 8-bit JPEG of the whole image

The preview SHALL be an 8-bit baseline JPEG, written at quality 90, with the
same width and height in pixels as the image it was made from. It SHALL hold
the whole image, uncropped and unscaled. A grayscale image SHALL give a
grayscale JPEG; every other image SHALL give a color JPEG. The preview
SHALL NOT apply an EXIF orientation.

#### Scenario: The preview keeps the size
- **WHEN** core runs the preview Operator over a stored PNG version of 4,096 by 2,160 pixels
- **THEN** its preview decodes as a JPEG of 4,096 by 2,160 pixels

#### Scenario: A grayscale image
- **WHEN** core runs the preview Operator over a stored 8-bit grayscale TIFF version
- **THEN** its preview decodes as a JPEG with one color component

#### Scenario: A JPEG with an EXIF rotation tag
- **WHEN** core runs the preview Operator over a stored JPEG version of 6,000 by 4,000 pixels whose EXIF orientation says to rotate it 90 degrees
- **THEN** its preview decodes as a JPEG of 6,000 by 4,000 pixels

### Requirement: The preview Operator decodes PNG, JPEG and TIFF

The preview Operator SHALL read the image by the version's media type,
which the Source set from the file's bytes, never by a file name. It SHALL
decode a PNG of any color type and bit
depth, an 8-bit baseline or progressive JPEG, and the first image of a TIFF
of a type that "The preview Operator decodes these TIFF types" lists. It
SHALL ignore every later image in a TIFF.

#### Scenario: A TIFF with several images
- **WHEN** core runs the preview Operator over a stored TIFF version whose first image is 800 by 600 pixels and whose second image is 200 by 150 pixels
- **THEN** its preview decodes as a JPEG of 800 by 600 pixels

#### Scenario: A progressive JPEG
- **WHEN** core runs the preview Operator over a stored progressive JPEG version of 1,920 by 1,080 pixels
- **THEN** its preview decodes as a JPEG of 1,920 by 1,080 pixels

#### Scenario: A JPEG exceeds the header read limit after its frame header
- **WHEN** core runs the preview Operator over a valid JPEG whose header fits the header read limit and whose full file is larger than 8 MiB
- **THEN** the run is `succeeded` and its preview keeps the width and height of the image

### Requirement: The preview Operator decodes these TIFF types

The preview Operator SHALL decode the first image in classic TIFF or BigTIFF
of either byte order when it has interleaved grayscale or
RGB samples in strips or tiles. It SHALL support unsigned 8-bit or 16-bit
integers and 32-bit floats, uncompressed or with LZW, Deflate or PackBits.
A predictor SHALL be absent or suit the codec and samples: horizontal
for unsigned integers, floating-point for floats.

#### Scenario: A big-endian BigTIFF
- **WHEN** core runs the preview Operator over a stored big-endian BigTIFF version that holds one 16-bit RGB image, compressed with Deflate
- **THEN** its run is `succeeded` and the version has a preview

#### Scenario: A white-is-zero TIFF
- **WHEN** core runs the preview Operator over a white-is-zero 8-bit grayscale TIFF with regions whose sample values are 0 and 255
- **THEN** the preview decodes to regions within 8 of white and black, respectively

#### Scenario: An LZW TIFF with a predictor
- **WHEN** core runs the preview Operator over a stored little-endian TIFF version that holds one 8-bit RGB image in tiles, compressed with LZW with a horizontal predictor
- **THEN** its run is `succeeded` and the version has a preview

#### Scenario: A floating-point TIFF with a padded edge tile
- **WHEN** core runs the preview Operator over a Deflate-compressed floating-point TIFF with a floating-point predictor and an edge tile wider than the image, within the decode budget
- **THEN** the run is `succeeded` and its preview keeps the width and height of the image

### Requirement: Sample values are scaled to 8-bit with no brightening

After interpreting the image's photometric polarity, the preview Operator
SHALL turn each sample into an 8-bit value as follows, and SHALL apply no
other change of brightness or contrast:

- an 8-bit sample stays as it is; a PNG sample of fewer bits is scaled so
  its largest value becomes 255;
- a 16-bit sample `v` becomes `v × 255 / 65535`, rounded;
- a 32-bit decimal `v` becomes `v × 255`, rounded, after a value below 0 is
  taken as 0 and one above 1 as 1; a value that is not a number becomes 0.

#### Scenario: A 16-bit TIFF
- **WHEN** core runs the preview Operator over a stored 16-bit grayscale TIFF version whose pixels are 0, 32768 and 65535
- **THEN** its preview's pixels decode within 8 of 0, 128 and 255

#### Scenario: A 32-bit decimal TIFF
- **WHEN** core runs the preview Operator over a stored 32-bit decimal grayscale TIFF version whose pixels are -0.5, 0.5, 2.0 and a value that is not a number
- **THEN** its preview's pixels decode within 8 of 0, 128, 255 and 0

#### Scenario: Unstretched data stays dark
- **WHEN** core runs the preview Operator over a stored 16-bit RGB TIFF version whose every sample is at most 1,000
- **THEN** every sample of its preview decodes within 8 of 4 or below

### Requirement: A transparent pixel shows over black

When a pixel has an alpha sample, the preview Operator SHALL show it over
black: each color sample becomes its value times the alpha, divided by the
largest alpha value.

#### Scenario: A transparent PNG pixel
- **WHEN** core runs the preview Operator over a stored 8-bit RGBA PNG version whose pixels are white with alpha 0, and orange (255, 128, 0) with alpha 255
- **THEN** its preview's pixels decode within 8 of black and of (255, 128, 0)

### Requirement: A TIFF of another type fails the preview run

When a TIFF's first image is of a type that "The preview Operator decodes
these TIFF types" does not list, the preview Operator SHALL decode no pixel.
Core SHALL record its run `failed` with the reason
`the TIFF image type is not supported`, and commit no Asset.

#### Scenario: A CMYK TIFF
- **WHEN** core runs the preview Operator over a stored TIFF version whose first image is CMYK
- **THEN** its run is `failed` with the reason `the TIFF image type is not supported`
- **AND** the version has no preview

#### Scenario: A CMYK TIFF is also over the nominal pixel budget
- **WHEN** core runs the preview Operator over a 16,000 by 12,000 TIFF whose first image is 8-bit CMYK
- **THEN** its run is `failed` with the reason `the TIFF image type is not supported`

#### Scenario: A TIFF compressed with JPEG
- **WHEN** core runs the preview Operator over a stored TIFF version whose first image is 8-bit RGB compressed with JPEG
- **THEN** its run is `failed` with the reason `the TIFF image type is not supported`

#### Scenario: An RGB TIFF with an alpha sample
- **WHEN** core runs the preview Operator over a stored TIFF version whose first image is 8-bit RGB with a fourth, alpha sample
- **THEN** its run is `failed` with the reason `the TIFF image type is not supported`

#### Scenario: A signed 16-bit TIFF
- **WHEN** core runs the preview Operator over a TIFF whose first image uses signed 16-bit samples
- **THEN** its run is `failed` with the reason `the TIFF image type is not supported`

### Requirement: A damaged image fails the preview run

When the preview Operator cannot decode an image, core SHALL record its run
`failed`, commit no Asset and change no other run. The reason SHALL be
`the <FORMAT> image could not be decoded`, where `<FORMAT>` is `PNG`,
`JPEG` or `TIFF` by the version's media type, for a file that is cut off,
damaged, refused by the decoder or not of that format. When a read of the
stored file fails, the reason SHALL be `the stored file could not be read`.

#### Scenario: A TIFF cut off inside its pixels
- **WHEN** core runs the preview Operator over a stored 8-bit RGB TIFF version whose last strip is cut off
- **THEN** its run is `failed` with the reason `the TIFF image could not be decoded`

#### Scenario: A PNG cut off after its header
- **WHEN** core runs the preview Operator over a stored version that holds a PNG's signature and header chunk and nothing after
- **THEN** its run is `failed` with the reason `the PNG image could not be decoded`

#### Scenario: A JPEG with no readable frame header
- **WHEN** core runs the preview Operator over a stored version of 1,000 bytes that starts with the JPEG signature but has no readable frame header
- **THEN** its run is `failed` with the reason `the JPEG image could not be decoded`

#### Scenario: A progressive JPEG cut off inside its pixels
- **WHEN** core runs the preview Operator over a progressive JPEG that ends in the middle of a scan, with no end-of-image marker
- **THEN** its run is `failed` with the reason `the JPEG image could not be decoded`

#### Scenario: A baseline JPEG ends before its end marker
- **WHEN** core runs the preview Operator over a baseline JPEG that ends without its end-of-image marker
- **THEN** its run is `failed` with the reason `the JPEG image could not be decoded`

#### Scenario: A PNG has pixels but no IEND
- **WHEN** core runs the preview Operator over a PNG whose complete image data is followed by an ancillary chunk and no IEND
- **THEN** its run is `failed` with the reason `the PNG image could not be decoded`

#### Scenario: Bytes that do not match the media type
- **WHEN** core runs the preview Operator over a stored version whose media type is `image/tiff` and whose bytes are a PNG
- **THEN** its run is `failed` with the reason `the TIFF image could not be decoded`
- **AND** the version has no preview

#### Scenario: The stored file cannot be read
- **WHEN** core runs the preview Operator over a stored PNG version and a read of its stored file fails
- **THEN** its run is `failed` with the reason `the stored file could not be read`
- **AND** the version has no preview

### Requirement: A JPEG that reaches its end marker gets a best-effort preview

A JPEG whose first image reaches its EOI SHALL get a preview even when its
scan data stops before every block it declares. The decoder fills the
missing blocks, as browsers do, and the original stays stored unchanged.

#### Scenario: A JPEG whose scan data stops short before its EOI
- **WHEN** core runs the preview Operator over a baseline JPEG of 800 by 600 pixels whose last scan loses its final bytes but still ends in EOI
- **THEN** its run is `succeeded` and its preview decodes as a JPEG of 800 by 600 pixels

### Requirement: A preview's sides fit a JPEG

A browser-ready JPEG preview SHALL have at most 65,500 pixels on each side.
For a wider or taller image, the preview Operator SHALL decode no pixel, and
core SHALL record its run `failed` with the reason
`the image is wider or taller than a browser-ready JPEG can hold`. This check
SHALL come before the memory limit.

#### Scenario: A TIFF wider than a JPEG can hold
- **WHEN** core runs the preview Operator over a stored 8-bit grayscale TIFF version whose header gives 65,501 by 100 pixels
- **THEN** its run is `failed` with the reason `the image is wider or taller than a browser-ready JPEG can hold`

#### Scenario: A TIFF at the browser-ready JPEG side limit
- **WHEN** core runs the preview Operator over a valid 8-bit grayscale TIFF version of 65,500 by 100 pixels within the decode budget
- **THEN** the run is `succeeded` and its JPEG preview is 65,500 by 100 pixels

### Requirement: A preview decode needs at most 512 MiB

The preview Operator SHALL limit each decode to 512 MiB (536,870,912 bytes),
including pixels, metadata and scratch space. If the header shows that
the decoded pixels cannot fit, it SHALL reject the image before pixel
decoding. During decoding, it SHALL refuse any allocation that would exceed the budget.
Core SHALL record the run `failed` with the reason
`the image needs more than 512 MiB to decode`.

#### Scenario: A TIFF fits the budget despite its large output buffer
- **WHEN** core runs the preview Operator over a valid 16,384 by 6,400 RGB8 TIFF whose decode fits the budget
- **THEN** the preview run is `succeeded` and the version has a preview at the original dimensions

#### Scenario: A TIFF just over the limit
- **WHEN** core runs the preview Operator over a stored 16-bit RGB TIFF version whose header gives 15,000 by 6,000 pixels (540,000,000 bytes of decoded pixels)
- **THEN** its run is `failed` with the reason `the image needs more than 512 MiB to decode`
- **AND** the preview Operator did not start pixel decoding

#### Scenario: TIFF pixels leave no space for decoding
- **WHEN** core runs the preview Operator over an 8-bit grayscale TIFF whose pixels alone need 512 MiB and whose decoder needs additional scratch memory
- **THEN** its run is `failed` with the reason `the image needs more than 512 MiB to decode`
- **AND** the preview Operator did not start pixel decoding

#### Scenario: TIFF tile scratch space exceeds the decode budget
- **WHEN** core runs the preview Operator over a grayscale floating-point TIFF whose padded tile scratch would exceed the decode budget
- **THEN** its run is `failed` with the reason `the image needs more than 512 MiB to decode`
- **AND** no preview of that version exists

#### Scenario: A PNG over the limit
- **WHEN** core runs the preview Operator over a stored 8-bit RGBA PNG version whose header gives 20,000 by 10,000 pixels (800,000,000 bytes of decoded pixels)
- **THEN** its run is `failed` with the reason `the image needs more than 512 MiB to decode`

#### Scenario: A JPEG over the limit
- **WHEN** core runs the preview Operator over a stored 8-bit color JPEG version whose frame header gives 60,000 by 60,000 pixels
- **THEN** its run is `failed` with the reason `the image needs more than 512 MiB to decode`

### Requirement: A decoder failure leaves the server available

When a preview decoder exceeds its budget or stops unexpectedly, the server
SHALL keep the original and finish the preview run as failed. It SHALL
accept later requests and SHALL NOT publish a partial preview.

#### Scenario: A preview fails after staging has begun
- **WHEN** a preview decoder stops unexpectedly after returning part of a JPEG
- **THEN** core records its preview run as failed and publishes no preview for the original
- **AND** a later valid upload can complete with a preview

### Requirement: A stalled preview run releases the preview slot

The preview Operator SHALL stop a decoder that has not exited after two
minutes. Core SHALL record the preview run as `failed` with the reason
`the preview decoder timed out`. The server SHALL keep the original and
allow the next preview to run.

#### Scenario: A stalled preview decoder
- **WHEN** a preview decoder is still alive at its deadline, even if its output has closed
- **THEN** the preview run is `failed` with the timeout reason
- **AND** a later valid upload completes with a preview

#### Scenario: Staging blocks while a decoder times out
- **WHEN** a preview decoder reaches its deadline while a staging write is blocked
- **THEN** the server reaps the decoder and lets a later decoder run before the first write returns
- **AND** when the first write returns successfully, core records the first preview run as `failed` with the timeout reason and publishes no preview

### Requirement: A preview gets no runs of its own

Core and the server SHALL run no Operator over a preview's version, because
no Operator accepts a derived Asset's version, as the asset-facets
capability says. A preview SHALL have no facet values and no preview of its
own.

#### Scenario: The preview of a new upload
- **WHEN** Nova uploads `nebula.png` and its preview run succeeds
- **THEN** no operation run exists for the preview's version, and the preview has no facet value and no preview

#### Scenario: An Operator asked to run over a preview
- **WHEN** core is asked to run the image pack's header Operator, and then its preview Operator, over the version of the preview of `nebula.png`, whose media type is `image/jpeg`
- **THEN** core records no operation run for either, and the preview has no facet value and no preview
