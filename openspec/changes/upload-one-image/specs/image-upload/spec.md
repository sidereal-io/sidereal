## Purpose

Let Nova bring one PNG or JPEG from their computer into the library through
the public Source contract and see the resulting image while signed in.

## ADDED Requirements

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

### Requirement: Upload accepts exactly one bounded image

`POST /api/assets` SHALL accept one multipart file field named `file`, with a
nonempty display filename. A file of at most 104,857,600 bytes (100 MiB) SHALL
be eligible for acceptance. A larger file SHALL return 413 with a clear
100 MiB limit message and create no Asset. Malformed multipart data, no file,
an empty filename, or more than one field SHALL return 400 and create no Asset.
A successful first upload SHALL return 201 with an `asset` object using the
same entry fields as the library list.

#### Scenario: One PNG
- **WHEN** Nova sends one recognized PNG below the limit
- **THEN** the response is 201 and identifies the committed Asset and original URL

#### Scenario: Exact size boundary
- **WHEN** Nova sends a recognized image of exactly 104,857,600 bytes
- **THEN** the size limit permits the upload

#### Scenario: One byte above the limit
- **WHEN** Nova sends a file of 104,857,601 bytes, including a streamed request without Content-Length
- **THEN** the response is 413 with the limit message and no Asset or version is created

#### Scenario: More than one file
- **WHEN** a request contains two file fields
- **THEN** the response is 400 and neither file creates an Asset

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

The signed-in view SHALL show an upload control for selecting one PNG or JPEG
and an `Upload` button. It SHALL show the selected filename, disable duplicate
submission while a request is in progress, and show each committed image by
its name and original. Successful upload SHALL refresh the library without a
page reload. Names SHALL render as text. The view SHALL retain existing
health, username, sign-out and event-socket behavior and follow DESIGN.md.

#### Scenario: First upload appears
- **WHEN** Nova chooses `nebula.png` and presses Upload and the server accepts it
- **THEN** the library shows `nebula.png` and its image without reloading

#### Scenario: Library after restart
- **WHEN** Nova opens the signed-in view after the server restarts with the same database and storage
- **THEN** the library loads and displays the saved image by name

#### Scenario: Upload in progress
- **WHEN** an upload request is pending
- **THEN** the view shows an uploading state and a second press cannot send another request

#### Scenario: Filename contains markup
- **WHEN** the library contains an image whose display name includes HTML markup
- **THEN** the name appears as literal text and executes no markup

### Requirement: Library failures are visible and recoverable

The library SHALL distinguish loading, empty, loaded and failed-list states.
Upload refusals SHALL show a clear message for unsupported format, excessive
size, invalid input or unavailable service, without claiming success. A failed
image load SHALL keep its name visible and show `This image could not be displayed.`
The user SHALL be able to retry a failed list request or select another file
after a refused upload. Signing out or losing the session SHALL hide library
data; late responses SHALL NOT restore it.

#### Scenario: Image cannot render
- **WHEN** an accepted image fails to load or decode in the browser
- **THEN** its name remains visible beside `This image could not be displayed.`

#### Scenario: Failed listing is not an empty library
- **WHEN** the list request fails or returns an invalid body
- **THEN** the view shows a library error and retry control, not the empty-library message

#### Scenario: Upload is refused
- **WHEN** upload returns 413 or 415
- **THEN** Nova sees the relevant limit or supported-format message and can select another file

#### Scenario: Session ends with work pending
- **WHEN** Nova signs out or the session ends while a library request is pending
- **THEN** the library disappears and a later response cannot display its contents again
