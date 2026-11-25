<!-- Plain language: Name the actor, condition and observable result. Explain necessary domain terms using the agreed glossary. Keep requirement and scenario headings and WHEN/THEN syntax exactly as required. -->

## MODIFIED Requirements

### Requirement: Manual upload executes a registered Source

Manual upload SHALL execute a Source obtained from the public Registry. The
Source SHALL recognize PNG, JPEG and TIFF through mediated access to the
candidate's bytes and propose an Asset through a run-scoped Asset context.
The contract SHALL give the Source no database handle, writable store path,
or authority to mint persisted identities or set a trusted content hash.
Rejected or failed Source execution SHALL create no Asset.

#### Scenario: Registered implementation is invoked
- **WHEN** a host registers a test Source through the same public contract and executes an upload through it
- **THEN** that Source observes the candidate through the context and its accepted proposal reaches core's ingest path
- **AND** the persisted provenance names the executing Source

#### Scenario: Source refuses a candidate
- **WHEN** the selected Source returns a refusal
- **THEN** no Asset or Asset version is committed

### Requirement: Image recognition uses the bytes

The manual-upload Source SHALL accept recognized PNG, JPEG or TIFF bytes
regardless of filename extension or client media type. A TIFF is recognized
by the first four bytes of a classic TIFF or a BigTIFF, in either byte
order. Its media type SHALL be `image/tiff`. The Source SHALL reject an
unrecognized format with status 415 and the message
`unsupported format: only PNG, JPEG and TIFF images are supported`, before
creating any Asset. Recognition SHALL NOT require full image decoding.

#### Scenario: Misleading extension
- **WHEN** Nova uploads recognized PNG bytes with filename `nebula.txt` and client type `text/plain`
- **THEN** the upload is accepted with PNG format

#### Scenario: A TIFF in each layout
- **WHEN** Nova uploads four files that start with `II*` and a zero byte, `MM`, a zero byte and `*`, `II+` and a zero byte, and `MM`, a zero byte and `+`
- **THEN** each upload is accepted, and each Asset's original has the media type `image/tiff`

#### Scenario: Unsupported bytes
- **WHEN** Nova uploads GIF, SVG, empty bytes or plain text named `nebula.tif`
- **THEN** the response is 415 with the message `unsupported format: only PNG, JPEG and TIFF images are supported`, and creates no Asset

#### Scenario: Recognizable but damaged image
- **WHEN** a candidate has a recognized PNG, JPEG or TIFF signature but cannot fully decode
- **THEN** recognition alone does not refuse the upload

### Requirement: The signed-in library uploads the files Nova picks

The Library SHALL show one `Upload` button that opens the file picker
for one or more PNG, JPEG or TIFF files. Choosing files SHALL start their
upload at once, as "The chosen files go one at a time" says. Each committed
image SHALL show with its name, as text, and its image, as "The Library
shows each image through its preview" says. Each saved file SHALL refresh
the library without a page reload. Health, username, sign-out and
event-socket behavior SHALL stay as they are, and the view SHALL follow
DESIGN.md. An empty library SHALL show the line
`No images yet. Upload a PNG, JPEG or TIFF to start.`

#### Scenario: First upload appears
- **WHEN** Nova presses Upload, chooses `nebula.png`, and the server accepts it
- **THEN** the library shows `nebula.png` and its image without reloading

#### Scenario: Several new images appear
- **WHEN** Nova picks `nebula.png` and `m31.jpg` in one pick, and the server accepts both
- **THEN** the library shows both images by name without reloading

#### Scenario: The picker offers TIFF
- **WHEN** Nova presses Upload
- **THEN** the file picker offers PNG, JPEG and TIFF files

#### Scenario: An empty library
- **WHEN** Nova opens the signed-in view of a library with no Assets
- **THEN** the Library shows `No images yet. Upload a PNG, JPEG or TIFF to start.`

#### Scenario: Upload in progress
- **WHEN** an upload request is pending and the Library shows
- **THEN** the Library shows the chosen filename as uploading, and neither a second press nor a second choice sends another request

#### Scenario: Library after restart
- **WHEN** Nova opens the signed-in view after the server restarts with the same database and storage
- **THEN** the library loads and displays the saved image by name

#### Scenario: Filename contains markup
- **WHEN** the library contains an image whose display name includes HTML markup
- **THEN** the name appears as literal text and executes no markup

### Requirement: A client that goes away does not stop the save or the run

Once the upload has passed its last cancellation check, a client that closes
its connection SHALL NOT stop the Asset's save, its file-size run, its
header run or its preview run. Each run SHALL end as it would have with the
client connected.

#### Scenario: The client goes away while the Asset is being saved
- **WHEN** the client closes its connection after the upload has passed its last cancellation check and before the Asset's transaction commits
- **THEN** the Asset is saved, one file-size run, one header run and one preview run are recorded for its version, each with outcome `succeeded`, and reading the Asset afterwards returns its file size, format, width and height and a `preview_url`

#### Scenario: The client goes away during the run
- **WHEN** the client closes its connection after the Asset is saved and before its upload runs have ended
- **THEN** the file-size run, the header run and the preview run all end `succeeded`, as they would have with the client connected, and reading the Asset afterwards returns its file size, format, width and height and a `preview_url`

### Requirement: A refused file's line says why

A refused file's line SHALL read `<name> was not uploaded: <reason>`. The
`<reason>` SHALL be `it is larger than the 100 MiB limit.` for 413,
`only PNG, JPEG and TIFF images are supported.` for 415,
`choose one PNG, JPEG or TIFF file and try again.` for 400, and
`the server could not save it. Try again later.` for any other refusal.

#### Scenario: Each kind of refusal
- **WHEN** Nova picks `a.png`, `b.png`, `c.png` and `d.png`, and the server answers 413, 415, 400 and 503
- **THEN** the lines read `a.png was not uploaded: it is larger than the 100 MiB limit.`, `b.png was not uploaded: only PNG, JPEG and TIFF images are supported.`, `c.png was not uploaded: choose one PNG, JPEG or TIFF file and try again.` and `d.png was not uploaded: the server could not save it. Try again later.`, in that order

### Requirement: One file's answer does not stop the others

A refusal, a 409, or an upload that ends without an answer SHALL NOT stop the
upload of the files after it in the same pick. Each of those files SHALL be
sent as usual.

#### Scenario: Every kind of answer in one pick
- **WHEN** Nova picks `a.png`, `b.png`, `c.png` and `d.png`, and the server answers 415 for `a.png`, 409 for `b.png`, nothing for `c.png` and 201 for `d.png`
- **THEN** four upload requests are sent, one for each file, in the order picked
- **AND** the screen shows `a.png was not uploaded: only PNG, JPEG and TIFF images are supported.`, `b.png is already in the library.`, `No answer came back for c.png. It is safe to upload it again: if it was saved, Sidereal will find it.` and `d.png was uploaded.`, in that order

### Requirement: A failed Operator run does not fail the upload

When an upload's file-size, header or preview run fails, its Operator stops
abnormally, or core refuses what it proposed, the upload SHALL still answer
201 with the saved Asset. Core SHALL record the run as failed, with its
reason and none of its values or Assets, when it can record the outcome.
When it cannot, the upload SHALL still answer 201, keep the Asset and its
version, and go on to the next run. No run SHALL stop the next run or remove
another run's values, and the server SHALL go on answering requests.

#### Scenario: The header run fails
- **WHEN** Nova uploads a file of 1,000 bytes that starts with the JPEG signature but has no readable frame header
- **THEN** the response is 201, the Library lists the image, and reading the Asset returns only the facet `image.file-size`, with the value 1000
- **AND** the header run is `failed` with the reason `the JPEG header could not be read`

#### Scenario: The file-size run fails
- **WHEN** Nova uploads a PNG and the file-size run fails
- **THEN** the response is 201, the Library lists the image, and reading the Asset returns no `image.file-size` facet
- **AND** the file-size run is `failed` with a reason, and a header run and a preview run are still recorded for its version

#### Scenario: The preview run fails
- **WHEN** Nova uploads a TIFF whose first image is CMYK, 800 by 600 pixels
- **THEN** the response is 201, its `asset.preview_url` is `null`, and reading the Asset returns `image.format` `TIFF`, `image.width` 800 and `image.height` 600
- **AND** the preview run is `failed` with the reason `the TIFF image type is not supported`

#### Scenario: A run's Operator panics
- **WHEN** Nova uploads a PNG and an Operator that upload runs panics
- **THEN** the response is 201 and that operation run is `failed` with a reason that says the Operator stopped
- **AND** the server answers the next request

#### Scenario: A run cannot be recorded
- **WHEN** Nova uploads a PNG and PostgreSQL refuses the runs' records after the Asset is saved
- **THEN** the response is 201 and the Asset and its version remain

#### Scenario: A failed run whose outcome cannot be recorded
- **WHEN** Nova uploads a file of 1,000 bytes that starts with the JPEG signature but has no readable frame header, and PostgreSQL refuses every update to the runs' outcomes after the Asset is saved
- **THEN** the response is 201, the Asset and its version remain, and reading the Asset returns no `image.format`, `image.width` or `image.height` facet
- **AND** the server log has a line that starts `image.read-header run not recorded for asset <id>:`

## ADDED Requirements

### Requirement: Upload makes a preview of each new image

After the header run ends, and before it answers, the upload SHALL run the
image pack's preview Operator over the new version, with the uploading
administrator as the user who started the run. The 201 answer's `asset`
SHALL hold the `preview_url` that reading the Asset gives after that run. A
repeat upload, answered 409, SHALL run no Operator. Images saved before this
change SHALL get no preview.

#### Scenario: A TIFF upload gets its preview
- **WHEN** Nova uploads `m42.tif`, an 8-bit RGB TIFF of 800 by 600 pixels, and its preview run succeeds
- **THEN** the response is 201, and its `asset.preview_url` is `/api/assets/<id>/preview` for the new Asset
- **AND** one operation run of the preview Operator exists for its version, with outcome `succeeded`, started by Nova

#### Scenario: A PNG upload gets its preview too
- **WHEN** Nova uploads `nebula.png` and the server answers 201
- **THEN** its `asset.preview_url` is `/api/assets/<id>/preview`, and the preview's bytes are a JPEG

#### Scenario: The runs go in order
- **WHEN** Nova uploads `nebula.png`
- **THEN** the file-size run ends before the header run starts, and the header run ends before the preview run starts

#### Scenario: An image saved before this change
- **WHEN** the server starts with this change and reads an Asset saved before it
- **THEN** its `preview_url` is `null`, and no preview run exists for its version

### Requirement: The Library shows each image through its preview

The Library SHALL show each image from its `preview_url`. When an image's
`preview_url` is `null`, the Library SHALL show it from its `original_url`.
When the browser cannot show that image, "An image that cannot display keeps
its name" applies.

#### Scenario: A TIFF in the Library
- **WHEN** the library holds `m42.tif`, whose preview run succeeded
- **THEN** the Library shows its image from `/api/assets/<id>/preview` and its name `m42.tif`

#### Scenario: A PNG saved before this change
- **WHEN** the library holds `nebula.png`, saved before this change, whose `preview_url` is `null`
- **THEN** the Library shows its image from `/api/assets/<id>/original`

#### Scenario: A TIFF whose preview run failed
- **WHEN** the library holds `cmyk.tif`, whose preview run failed, and the browser cannot show its original
- **THEN** the Library shows the name `cmyk.tif` beside `This image could not be displayed.`

#### Scenario: A preview address from elsewhere
- **WHEN** the list gives an Asset a `preview_url` other than `/api/assets/<id>/preview` for that Asset's own id, or a value that is neither text nor `null`
- **THEN** the Library treats the list as not valid, and shows its library error and Retry control
