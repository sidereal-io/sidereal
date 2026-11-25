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

The signed-in view SHALL show one `Upload` button that opens the file picker
for one PNG or JPEG. Choosing a file SHALL start its upload at once.
While it is in progress, the view SHALL show the chosen filename and send no
second request. Each committed image SHALL show with its name, as text, and
its original. A successful upload SHALL refresh the library without a page
reload. Health, username, sign-out and event-socket behavior SHALL stay as
they are, and the view SHALL follow DESIGN.md.

#### Scenario: First upload appears
- **WHEN** Nova presses Upload, chooses `nebula.png`, and the server accepts it
- **THEN** the library shows `nebula.png` and its image without reloading

#### Scenario: Library after restart
- **WHEN** Nova opens the signed-in view after the server restarts with the same database and storage
- **THEN** the library loads and displays the saved image by name

#### Scenario: Upload in progress
- **WHEN** an upload request is pending
- **THEN** the view shows the chosen filename as uploading, and neither a second press nor a second choice sends another request

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

### Requirement: A repeat upload answers 409 with the existing Asset

When ingest finds the existing Asset for an upload, as the asset-library
capability defines, `POST /api/assets` SHALL answer 409 Conflict with a JSON
body holding an `asset` object for the found Asset. The object SHALL use the
same entry fields as the library list and the 201 answer: `id`, `kind`,
`name`, `current_version_id` and `original_url`. A first upload SHALL still
answer 201. A failed publication SHALL still answer 503, not 409.

#### Scenario: The same PNG again
- **WHEN** Nova uploads `nebula.png`, and then uploads the same bytes as `m31-final.png`
- **THEN** the first answer is 201 and the second is 409
- **AND** the 409 body's `asset` has the first answer's `id`, `current_version_id` and `original_url`, and `name` `nebula.png`

#### Scenario: A refused upload is not a repeat
- **WHEN** Nova uploads plain text named `nebula.png`
- **THEN** the answer is 415, as before, and not 409

#### Scenario: The stored object of the existing Asset is damaged
- **WHEN** Nova uploads `nebula.png`, its stored object's bytes then change, and Nova uploads the same bytes again
- **THEN** the second answer is 503, as before, and not 409
- **AND** the library still holds one Asset and one Asset version, and the stored object is unchanged

### Requirement: The library screen says when an image is already there

When an upload answers 409, the library screen SHALL show a notice, not a
refusal: `<chosen name> is already in the library.` When the answer's
`asset` has a valid entry whose name differs from the chosen filename, the
notice SHALL read `<chosen name> is already in the library as <existing name>.`
When the body holds no valid entry, the screen SHALL show the shorter notice.
Both names SHALL appear as text only.

#### Scenario: The same name
- **WHEN** Nova uploads `nebula.png`, and the server answers 409 with an Asset named `nebula.png`
- **THEN** the screen shows `nebula.png is already in the library.` and no `was not uploaded` message

#### Scenario: Another name
- **WHEN** Nova uploads `m31-final.png`, and the server answers 409 with an Asset named `nebula.png`
- **THEN** the screen shows `m31-final.png is already in the library as nebula.png.`

#### Scenario: A 409 body without a valid entry
- **WHEN** the server answers 409 with a body that holds no valid `asset` entry
- **THEN** the screen shows `<chosen name> is already in the library.` and no name from the body

### Requirement: After a repeat upload, another file can be chosen at once

When an upload answers 409, the library screen SHALL NOT load the list
again, and the user SHALL be able to choose and upload another file at once.

#### Scenario: Ready for the next file
- **WHEN** Nova uploads `nebula.png`, and the server answers 409
- **THEN** the Upload button is enabled and the list is not requested again

### Requirement: A lost upload answer says another upload is safe

When an upload request ends without an answer, the library screen SHALL show
`No answer came back for <chosen name>. It is safe to upload it again: if it was saved, Sidereal will find it.`
It SHALL load the list again, and it SHALL NOT send the upload again by
itself.

#### Scenario: The answer is lost after the server saved the file
- **WHEN** the upload request fails without an answer, and the reloaded list holds `nebula.png`
- **THEN** the screen shows `No answer came back for nebula.png. It is safe to upload it again: if it was saved, Sidereal will find it.`
- **AND** it shows `nebula.png` in the list, and exactly one upload request was sent
