<!-- Plain language: Name the actor, condition and observable result. Explain necessary domain terms using the agreed glossary. Keep requirement and scenario headings and WHEN/THEN syntax exactly as required. -->

## Purpose

Let the image pack make a browser-ready preview of a PNG, JPEG or TIFF
version, so the library can show an image the browser cannot show itself.

## ADDED Requirements

### Requirement: The image pack registers a preview Operator

The image pack SHALL register an Operator with the id `image.make-preview`,
separate from its file-size and header Operators. It SHALL declare that it
makes previews, and SHALL list no facets. Each successful run SHALL propose
one preview of kind `image` and media type `image/jpeg`.

#### Scenario: The preview Operator's registration
- **WHEN** the image pack registers
- **THEN** the Registry holds the Operator `image.make-preview`, which makes previews and lists no facets

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

The preview Operator SHALL find the format from the version's leading bytes,
never from a file name. It SHALL decode a PNG of any color type and bit
depth, an 8-bit baseline or progressive JPEG, and the first image of a TIFF
of a type that "The preview Operator decodes these TIFF types" lists. It
SHALL ignore every later image in a TIFF.

#### Scenario: A TIFF with several images
- **WHEN** core runs the preview Operator over a stored TIFF version whose first image is 800 by 600 pixels and whose second image is 200 by 150 pixels
- **THEN** its preview decodes as a JPEG of 800 by 600 pixels

#### Scenario: A progressive JPEG
- **WHEN** core runs the preview Operator over a stored progressive JPEG version of 1,920 by 1,080 pixels
- **THEN** its preview decodes as a JPEG of 1,920 by 1,080 pixels

### Requirement: The preview Operator decodes these TIFF types

The preview Operator SHALL decode a TIFF's first image, in a classic TIFF or
BigTIFF file of either byte order, when all of these hold:

- it is grayscale (black is zero) with one sample per pixel, or RGB with
  three;
- its samples are 8-bit or 16-bit whole numbers, or 32-bit decimals;
- its samples are interleaved, in strips or tiles;
- it is uncompressed, or compressed with LZW, Deflate or PackBits, with or
  without a horizontal or floating-point predictor.

#### Scenario: A big-endian BigTIFF
- **WHEN** core runs the preview Operator over a stored big-endian BigTIFF version that holds one 16-bit RGB image, compressed with Deflate
- **THEN** its run is `succeeded` and the version has a preview

#### Scenario: An LZW TIFF with a predictor
- **WHEN** core runs the preview Operator over a stored little-endian TIFF version that holds one 8-bit RGB image in tiles, compressed with LZW with a horizontal predictor
- **THEN** its run is `succeeded` and the version has a preview

### Requirement: Sample values are scaled to 8-bit with no brightening

The preview Operator SHALL turn each sample into an 8-bit value as follows,
and SHALL apply no other change of brightness or contrast:

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

#### Scenario: A TIFF compressed with JPEG
- **WHEN** core runs the preview Operator over a stored TIFF version whose first image is 8-bit RGB compressed with JPEG
- **THEN** its run is `failed` with the reason `the TIFF image type is not supported`

#### Scenario: An RGB TIFF with an alpha sample
- **WHEN** core runs the preview Operator over a stored TIFF version whose first image is 8-bit RGB with a fourth, alpha sample
- **THEN** its run is `failed` with the reason `the TIFF image type is not supported`

### Requirement: A damaged or unrecognized image fails the preview run

When the preview Operator cannot decode an image, core SHALL record its run
`failed`, commit no Asset and change no other run. The reason SHALL be
`the image format is not recognized` for bytes that start with no PNG, JPEG
or TIFF signature. It SHALL be `the <FORMAT> image could not be decoded`,
where `<FORMAT>` is `PNG`, `JPEG` or `TIFF` by signature, for a file that
is cut off, damaged or refused by the decoder.

#### Scenario: A TIFF cut off inside its pixels
- **WHEN** core runs the preview Operator over a stored 8-bit RGB TIFF version whose last strip is cut off
- **THEN** its run is `failed` with the reason `the TIFF image could not be decoded`

#### Scenario: A PNG cut off after its header
- **WHEN** core runs the preview Operator over a stored version that holds a PNG's signature and header chunk and nothing after
- **THEN** its run is `failed` with the reason `the PNG image could not be decoded`

#### Scenario: A JPEG with no readable frame header
- **WHEN** core runs the preview Operator over a stored version of 1,000 bytes that starts with the JPEG signature but has no readable frame header
- **THEN** its run is `failed` with the reason `the JPEG image could not be decoded`

#### Scenario: Bytes of no supported format
- **WHEN** core runs the preview Operator over a stored version whose bytes start with no PNG, JPEG or TIFF signature
- **THEN** its run is `failed` with the reason `the image format is not recognized`

### Requirement: A preview's sides fit a JPEG

A JPEG side holds at most 65,535 pixels. For an image wider or taller than
that, the preview Operator SHALL decode no pixel, and core SHALL record its
run `failed` with the reason
`the image is wider or taller than a JPEG can hold`. This check SHALL come
before the memory limit.

#### Scenario: A TIFF wider than a JPEG can hold
- **WHEN** core runs the preview Operator over a stored 8-bit grayscale TIFF version whose header gives 70,000 by 100 pixels
- **THEN** its run is `failed` with the reason `the image is wider or taller than a JPEG can hold`

### Requirement: A preview needs at most 512 MiB of decoded pixels

Before it decodes a pixel, the preview Operator SHALL work out the bytes the
decoded pixels need: width × height × samples per pixel × bytes per sample. A sample counts 1 byte at 8 bits or fewer, 2 at 16 bits and 4 as a
32-bit decimal. A PNG palette counts 3 samples, or 4 with transparency. A
progressive JPEG adds 2 bytes per sample. Over 512 MiB (536,870,912 bytes),
it SHALL decode no pixel, and core SHALL record the run `failed` with the
reason `the image needs more than 512 MiB to decode`.

#### Scenario: A TIFF just over the limit
- **WHEN** core runs the preview Operator over a stored 16-bit RGB TIFF version whose header gives 15,000 by 6,000 pixels (540,000,000 bytes of decoded pixels)
- **THEN** its run is `failed` with the reason `the image needs more than 512 MiB to decode`
- **AND** the Operator read no pixel data

#### Scenario: A TIFF exactly at the limit
- **WHEN** core runs the preview Operator over a stored 8-bit grayscale TIFF version whose header gives 32,768 by 16,384 pixels (536,870,912 bytes of decoded pixels) and whose pixel data is cut off
- **THEN** its run is `failed` with the reason `the TIFF image could not be decoded`, not the memory limit's reason

#### Scenario: A PNG over the limit
- **WHEN** core runs the preview Operator over a stored 8-bit RGBA PNG version whose header gives 20,000 by 10,000 pixels (800,000,000 bytes of decoded pixels)
- **THEN** its run is `failed` with the reason `the image needs more than 512 MiB to decode`

#### Scenario: A JPEG over the limit
- **WHEN** core runs the preview Operator over a stored 8-bit color JPEG version whose frame header gives 60,000 by 60,000 pixels
- **THEN** its run is `failed` with the reason `the image needs more than 512 MiB to decode`

### Requirement: A preview gets no runs of its own

Core and the server SHALL run no Operator over a preview's version when it
is made. A preview SHALL have no facet values and no preview of its own.

#### Scenario: The preview of a new upload
- **WHEN** Nova uploads `nebula.png` and its preview run succeeds
- **THEN** no operation run exists for the preview's version, and the preview has no facet value and no preview
