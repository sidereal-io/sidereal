# image-upload Specification

## Purpose
Let Nova bring one PNG or JPEG from their computer into the library through
the public Source contract and see the resulting image while signed in.

## Requirements

### Requirement: Manual upload executes a registered Source

Manual upload SHALL execute a Source obtained from the public Registry. The
Source SHALL recognize PNG and JPEG through mediated access to the candidate's
bytes and propose an Asset through a run-scoped Asset context. The contract
SHALL give the Source no database handle, writable store path, or authority to
mint persisted identities or set a trusted content hash. Rejected or failed
Source execution SHALL create no Asset.

#### Scenario: Registered implementation is invoked
- **WHEN** a host registers a test Source through the same public contract and executes an upload through it
- **THEN** that Source observes the candidate through the context and its accepted proposal reaches core's ingest path
- **AND** the persisted provenance names the executing Source

#### Scenario: Source refuses a candidate
- **WHEN** the selected Source returns a refusal
- **THEN** no Asset or Asset version is committed

### Requirement: Upload accepts exactly one file

`POST /api/assets` SHALL accept one multipart file field named `file`, with a
nonempty display filename. Malformed multipart data, no file, an empty
filename, or more than one field SHALL return 400 and create no Asset. A
successful first upload SHALL return 201 with an `asset` object using the same entry fields as
the library list.

#### Scenario: One PNG
- **WHEN** Nova sends one recognized PNG below the limit
- **THEN** the response is 201 and identifies the committed Asset and original URL

#### Scenario: More than one file
- **WHEN** a request contains two file fields
- **THEN** the response is 400 and neither file creates an Asset

### Requirement: Upload data ends at the closing delimiter

An upload's multipart data SHALL end at its closing delimiter. A body that
ends or fails before the closing delimiter SHALL return 400 and create no
Asset. Bytes after the closing delimiter are an epilogue: upload SHALL ignore
them, as RFC 2046 says. The whole request body, epilogue included, SHALL fit
the route's transport limit of 101 MiB (105,906,176 bytes). A larger body
SHALL return 413 and create no Asset.

#### Scenario: Body cut off before the closing delimiter
- **WHEN** a request holds a complete recognized PNG part, but the body ends before the closing delimiter
- **THEN** the response is 400, no Asset or version is committed, and the object store's file list is unchanged

#### Scenario: Epilogue after the closing delimiter
- **WHEN** a request holds one recognized PNG part and its closing delimiter, followed by a small epilogue that keeps the body within the transport limit
- **THEN** the response is 201 and the Asset's original is the PNG's bytes

### Requirement: Upload enforces a 100 MiB size limit

A file of at most 104,857,600 bytes (100 MiB) SHALL be eligible for
acceptance. A larger file SHALL return 413 with a clear 100 MiB limit message
and create no Asset, whether or not the request states its length.

#### Scenario: Exact size boundary
- **WHEN** Nova sends a recognized image of exactly 104,857,600 bytes
- **THEN** the size limit permits the upload

#### Scenario: One byte above the limit
- **WHEN** Nova sends a file of 104,857,601 bytes, including a streamed request without Content-Length
- **THEN** the response is 413 with the limit message and no Asset or version is created

### Requirement: Image recognition uses the bytes

The manual-upload Source SHALL accept recognized PNG or JPEG bytes regardless
of filename extension or client media type. It SHALL reject an unrecognized
format with status 415 and a message naming PNG and JPEG as supported formats,
before creating any Asset. Recognition SHALL NOT require full image decoding.

#### Scenario: Misleading extension
- **WHEN** Nova uploads recognized PNG bytes with filename `nebula.txt` and client type `text/plain`
- **THEN** the upload is accepted with PNG format

#### Scenario: Unsupported bytes
- **WHEN** Nova uploads TIFF, SVG, empty bytes or plain text named `nebula.jpg`
- **THEN** the response is 415, names PNG and JPEG, and creates no Asset

#### Scenario: Recognizable but damaged image
- **WHEN** a candidate has a recognized PNG or JPEG signature but cannot fully decode
- **THEN** recognition alone does not refuse the upload

### Requirement: Unavailable storage refuses upload clearly

When managed storage is not configured, or an ingest storage or database
operation fails before commit, upload SHALL return 503 with a clear failure
message. It SHALL NOT commit a partial Asset or version. Unset `STORAGE_ROOT`
SHALL continue to permit database-only health and readiness serving.

#### Scenario: Database-only server
- **WHEN** a signed-in administrator uploads to a server with no `STORAGE_ROOT`
- **THEN** upload returns 503 indicating storage is unavailable and creates no Asset
- **AND** health and readiness retain their existing behavior

### Requirement: The signed-in library supports one-image upload

The Library SHALL show one `Upload` button that opens the file picker
for one PNG or JPEG. Choosing a file SHALL start its upload at once.
While it is in progress, the Library SHALL show the chosen filename and send
no second request. Each committed image SHALL show with its name, as text,
and its original. A successful upload SHALL refresh the library without a
page reload. Health, username, sign-out and event-socket behavior SHALL stay
as they are, and the view SHALL follow DESIGN.md.

#### Scenario: First upload appears
- **WHEN** Nova presses Upload, chooses `nebula.png`, and the server accepts it
- **THEN** the library shows `nebula.png` and its image without reloading

#### Scenario: Library after restart
- **WHEN** Nova opens the signed-in view after the server restarts with the same database and storage
- **THEN** the library loads and displays the saved image by name

#### Scenario: Upload in progress
- **WHEN** an upload request is pending and the Library shows
- **THEN** the Library shows the chosen filename as uploading, and neither a second press nor a second choice sends another request

#### Scenario: Filename contains markup
- **WHEN** the library contains an image whose display name includes HTML markup
- **THEN** the name appears as literal text and executes no markup

### Requirement: Library states are distinct

The library SHALL distinguish loading, empty, loaded and failed-list states.
The user SHALL be able to retry a failed list request.

#### Scenario: Failed listing is not an empty library
- **WHEN** the list request fails or returns an invalid body
- **THEN** the view shows a library error and retry control, not the empty-library message

### Requirement: Upload refusals are clear and recoverable

Upload refusals SHALL show a clear message for unsupported format, excessive
size, invalid input or unavailable service, without claiming success. The user
SHALL be able to select another file after a refused upload.

#### Scenario: Upload is refused
- **WHEN** upload returns 413 or 415
- **THEN** Nova sees the relevant limit or supported-format message and can select another file

### Requirement: An image that cannot display keeps its name

A failed image load SHALL keep its name visible and show
`This image could not be displayed.`

#### Scenario: Image cannot render
- **WHEN** an accepted image fails to load or decode in the browser
- **THEN** its name remains visible beside `This image could not be displayed.`

### Requirement: Ending the session hides library data

Signing out or losing the session SHALL hide library data. Responses that
arrive later SHALL NOT restore it.

#### Scenario: Session ends with work pending
- **WHEN** Nova signs out or the session ends while a library request is pending
- **THEN** the library disappears and a later response cannot display its contents again

### Requirement: Upload records the image's file size

After a successful upload saves the Asset, and before it answers, the server
SHALL run the image pack's file-size Operator over the new version, with the
uploading administrator as the user who started the run. The 201 answer
SHALL keep its existing body. When the run succeeded, reading the Asset
after the 201 SHALL return its file size; when it did not, the requirement
"A failed file-size run does not fail the upload" applies. Images saved
before this change SHALL get no file size.

#### Scenario: A new upload has its file size
- **WHEN** Nova uploads a PNG of 1,234,567 bytes, its file-size run succeeds, and the server answers 201
- **THEN** reading that Asset returns the facet `image.file-size` with the value 1234567 and the unit `bytes`
- **AND** one operation run of the file-size Operator exists for its version, with outcome `succeeded`, started by Nova

### Requirement: A client that goes away does not stop the save or the run

Once the upload has passed its last cancellation check, a client that closes
its connection SHALL NOT stop the Asset's save or its file-size run. The run
SHALL end as it would have with the client connected.

#### Scenario: The client goes away while the Asset is being saved
- **WHEN** the client closes its connection after the upload has passed its last cancellation check and before the Asset's transaction commits
- **THEN** the Asset is saved, one file-size run is recorded for its version with outcome `succeeded`, and reading the Asset afterwards returns its file size

#### Scenario: The client goes away during the run
- **WHEN** the client closes its connection after the Asset is saved and while the file-size Operator is running
- **THEN** the run ends `succeeded`, as it would have with the client connected, and reading the Asset afterwards returns its file size

### Requirement: A failed file-size run does not fail the upload

When the file-size run fails, its Operator stops abnormally, core refuses
what it proposed, or the server cannot record the run, the upload SHALL
still answer 201 with the saved Asset. A failed run SHALL be recorded as
failed with its reason, and the Asset SHALL have no file size. The server
log SHALL name the failure, and SHALL say when the run's outcome could not
be recorded. The server SHALL go on answering requests.

#### Scenario: The run fails
- **WHEN** Nova uploads a PNG and the file-size run fails
- **THEN** the response is 201, the Library lists the image, and reading the Asset returns `"facets":[]`
- **AND** its operation run is `failed` with a reason
- **AND** the server log has the line `file-size run failed for asset <id>: <reason>`

#### Scenario: The run's Operator panics
- **WHEN** Nova uploads a PNG and the Operator that upload runs panics
- **THEN** the response is 201 and its operation run is `failed` with a reason that says the Operator stopped
- **AND** the server answers the next request

#### Scenario: The run cannot be recorded
- **WHEN** Nova uploads a PNG and PostgreSQL refuses the run's record after the Asset is saved
- **THEN** the response is 201 and the Asset and its version remain
- **AND** the server log has a line that starts `file-size run not recorded for asset <id>:`
