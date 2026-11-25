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

### Requirement: The signed-in library uploads the files Nova picks

The Library SHALL show one `Upload` button that opens the file picker
for one or more PNG or JPEG files. Choosing files SHALL start their upload at
once, as "The chosen files go one at a time" says. Each committed image SHALL
show with its name, as text, and its original. Each saved file SHALL refresh
the library without a page reload. Health, username, sign-out and
event-socket behavior SHALL stay as they are, and the view SHALL follow
DESIGN.md.

#### Scenario: First upload appears
- **WHEN** Nova presses Upload, chooses `nebula.png`, and the server accepts it
- **THEN** the library shows `nebula.png` and its image without reloading

#### Scenario: Several new images appear
- **WHEN** Nova picks `nebula.png` and `m31.jpg` in one pick, and the server accepts both
- **THEN** the library shows both images by name without reloading

#### Scenario: Upload in progress
- **WHEN** an upload request is pending and the Library shows
- **THEN** the Library shows the chosen filename as uploading, and neither a second press nor a second choice sends another request

#### Scenario: Library after restart
- **WHEN** Nova opens the signed-in view after the server restarts with the same database and storage
- **THEN** the library loads and displays the saved image by name

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
SHALL be able to select other files after a refused upload, once every file
in the same pick has an answer.

#### Scenario: Upload is refused
- **WHEN** Nova picks only `nebula.png`, and its upload returns 413 or 415
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

### Requirement: A repeat upload does not load the list again

When an upload answers 409, the library screen SHALL NOT load the list again
for that file. Once every file in the same pick has an answer, the user SHALL
be able to choose and upload other files.

#### Scenario: Ready for the next file
- **WHEN** Nova picks only `nebula.png`, and the server answers 409
- **THEN** the Upload button is enabled and the list is not requested again

#### Scenario: A repeat in the middle of a pick
- **WHEN** Nova picks `a.png`, `b.png` and `c.png`, and the test releases the answers 201, 409 and 201 one at a time: the answer for `b.png` only once the list request that the answer for `a.png` causes has started, and the answer for `c.png` only once the line for `b.png` shows
- **THEN** a list request starts after the answer for `a.png` and another after the answer for `c.png`
- **AND** no list request starts between the answer for `b.png` and the answer for `c.png`

### Requirement: A lost upload answer says another upload is safe

When an upload request ends without an answer, the library screen SHALL show
`No answer came back for <chosen name>. It is safe to upload it again: if it was saved, Sidereal will find it.`
It SHALL load the list again, and it SHALL NOT send the upload again by
itself.

#### Scenario: The answer is lost after the server saved the file
- **WHEN** the upload request fails without an answer, and the reloaded list holds `nebula.png`
- **THEN** the screen shows `No answer came back for nebula.png. It is safe to upload it again: if it was saved, Sidereal will find it.`
- **AND** it shows `nebula.png` in the list, and exactly one upload request was sent

### Requirement: The chosen files go one at a time

The library screen SHALL send the chosen files one at a time, in the order
picked, as one upload request each. It SHALL send a file only after the file
before it has an answer or has ended without one.

#### Scenario: Files go in turn
- **WHEN** Nova picks `a.png`, `b.png` and `c.png` in one pick, and the server answers each upload only when the test says so
- **THEN** exactly one upload request is pending at any time
- **AND** the requests carry `a.png`, then `b.png`, then `c.png`, each as the only file in its request

### Requirement: The list loads again after a saved file or a lost answer

After each saved file, and after each file whose upload ended without an
answer, the library screen SHALL start a list request without a page reload.
Answers that come back before that request starts MAY share it. A 409 SHALL
start none, as "A repeat upload does not load the list again" says.

#### Scenario: A lost answer in the middle of a pick
- **WHEN** Nova picks `a.png` and `b.png`, the upload of `a.png` ends without an answer, and the upload of `b.png` is pending
- **THEN** a list request has started after the lost answer for `a.png`, before `b.png` has its answer

### Requirement: Each chosen file has a line that shows its progress

The library screen SHALL show one line for each file in the current pick, in
the order picked. A file not yet sent SHALL read `<name> is waiting.` A file
on its way SHALL read `Uploading <name>…`. A saved file SHALL read
`<name> was uploaded.` Any other file SHALL read the message for its answer:
"A refused file's line says why", "The library screen says when an image is
already there" and "A lost upload answer says another upload is safe" define
them. Every name SHALL appear as text only.

#### Scenario: Lines while the second file uploads
- **WHEN** Nova picks `a.png`, `b.png` and `c.png`, the server accepts `a.png`, and the upload of `b.png` is pending
- **THEN** the screen shows `a.png was uploaded.`, `Uploading b.png…` and `c.png is waiting.`, in that order

#### Scenario: A waiting filename contains markup
- **WHEN** Nova picks `a.png` and `<b>m42</b>.png`, and the upload of `a.png` is pending
- **THEN** the screen shows `<b>m42</b>.png is waiting.` as literal text and no `b` element

### Requirement: A pick's lines stay until the next pick

Each line SHALL stay until Nova next picks files. The next pick SHALL replace
every line with the lines of its own files. A pick of one file SHALL follow
the same rules.

#### Scenario: One file's line stays after its answer
- **WHEN** Nova picks only `nebula.png`, and the server accepts it
- **THEN** the screen shows `nebula.png was uploaded.` and the image `nebula.png`

#### Scenario: The next pick replaces the lines
- **WHEN** every file in Nova's pick of `a.png` and `b.png` has an answer, and Nova then picks `c.png`
- **THEN** the screen shows a line for `c.png` and no line for `a.png` or `b.png`

### Requirement: A refused file's line says why

A refused file's line SHALL read `<name> was not uploaded: <reason>`. The
`<reason>` SHALL be `it is larger than the 100 MiB limit.` for 413,
`only PNG and JPEG images are supported.` for 415,
`choose one PNG or JPEG file and try again.` for 400, and
`the server could not save it. Try again later.` for any other refusal.
These are the words the screen shows for a refusal today.

#### Scenario: Each kind of refusal
- **WHEN** Nova picks `a.png`, `b.png`, `c.png` and `d.png`, and the server answers 413, 415, 400 and 503
- **THEN** the lines read `a.png was not uploaded: it is larger than the 100 MiB limit.`, `b.png was not uploaded: only PNG and JPEG images are supported.`, `c.png was not uploaded: choose one PNG or JPEG file and try again.` and `d.png was not uploaded: the server could not save it. Try again later.`, in that order

### Requirement: One file's answer does not stop the others

A refusal, a 409, or an upload that ends without an answer SHALL NOT stop the
upload of the files after it in the same pick. Each of those files SHALL be
sent as usual.

#### Scenario: Every kind of answer in one pick
- **WHEN** Nova picks `a.png`, `b.png`, `c.png` and `d.png`, and the server answers 415 for `a.png`, 409 for `b.png`, nothing for `c.png` and 201 for `d.png`
- **THEN** four upload requests are sent, one for each file, in the order picked
- **AND** the screen shows `a.png was not uploaded: only PNG and JPEG images are supported.`, `b.png is already in the library.`, `No answer came back for c.png. It is safe to upload it again: if it was saved, Sidereal will find it.` and `d.png was uploaded.`, in that order

### Requirement: Upload waits for the whole pick

From the moment Nova picks files until every file in that pick has an answer
or has ended without one, the Upload button SHALL be disabled, and neither a
press nor a new choice of files SHALL send a request. When the last file has
its answer, the Upload button SHALL be enabled again.

#### Scenario: A pick in progress
- **WHEN** Nova picks `a.png` and `b.png`, and the upload of `a.png` is pending
- **THEN** the Upload button is disabled, and a press or a new choice of `c.png` sends no request and adds no line

#### Scenario: The last answer enables Upload
- **WHEN** Nova picks `a.png` and `b.png`, and the server answers 201 for `a.png` and 415 for `b.png`
- **THEN** the Upload button is enabled once `b.png` has its answer, and not before

### Requirement: Ending the session stops the files not yet sent

Once the library screen knows the session has ended, it SHALL NOT send any
more files of the pick, even before the sign-in form shows. The screen knows
when Nova's sign-out has answered 204 or 401, when an upload or list request
answers 401, or when the events connection says the session ended. An upload
that answers 401 SHALL move the screen to the sign-in form. The library and
every file's line SHALL be hidden, as "Ending the session hides library data"
requires.

#### Scenario: An upload answers 401
- **WHEN** Nova picks `a.png`, `b.png` and `c.png`, and the upload of `a.png` answers 401
- **THEN** the screen moves to the sign-in form, and no upload request is sent for `b.png` or `c.png`

#### Scenario: The sign-out and an upload answer together
- **WHEN** Nova picks `a.png`, `b.png` and `c.png`, presses Sign out while the upload of `b.png` is pending, and the test releases the sign-out's 204 and then the answer 201 for `b.png` in one step, with no wait between them
- **THEN** no upload request is sent for `c.png`, and the screen shows the sign-in form

#### Scenario: The events connection ends the session during a pick
- **WHEN** Nova picks `a.png` and `b.png`, the events connection says the session ended while the upload of `a.png` is pending, and the upload of `a.png` then answers 201
- **THEN** no upload request is sent for `b.png`, and the screen shows the sign-in form

### Requirement: A pending sign-out does not stop the pick

Until the screen knows the session has ended, it SHALL go on sending the
files in turn, including while a sign-out is still pending. The server checks
the session once, when each request arrives. So a file it admits gets its
usual answer, and a file that arrives after the session is gone gets 401,
which ends the pick as "Ending the session stops the files not yet sent"
says. A sign-out that fails or gets no answer tells the screen nothing about
the session, so the pick SHALL go on.

#### Scenario: Nova signs out during a pick
- **WHEN** Nova picks `a.png`, `b.png` and `c.png`, presses Sign out while the upload of `a.png` is pending, `a.png` answers 201 while the sign-out is still pending, and the sign-out answers 204 while the upload of `b.png` is pending
- **THEN** the screen shows the sign-in form, and the library and the lines are gone
- **AND** upload requests were sent for `a.png` and `b.png` only, and no request is sent for `c.png`
- **AND** a later answer for `b.png` shows nothing and starts no list request

#### Scenario: A failed sign-out leaves the pick going
- **WHEN** Nova picks `a.png` and `b.png`, presses Sign out while the upload of `a.png` is pending, the sign-out answers 500, the uploads of `a.png` and `b.png` both answer 201, and the events connection stays open
- **THEN** the library stays, the upload request for `b.png` is sent once `a.png` has its answer, and the lines read `a.png was uploaded.` and `b.png was uploaded.`

#### Scenario: A sign-out with no answer leaves the pick going
- **WHEN** Nova picks `a.png` and `b.png`, presses Sign out while the upload of `a.png` is pending, the sign-out request ends without an answer, the uploads of `a.png` and `b.png` both answer 201, and the events connection stays open
- **THEN** the library stays, the upload request for `b.png` is sent once `a.png` has its answer, and the lines read `a.png was uploaded.` and `b.png was uploaded.`

#### Scenario: A sign-out with no answer, then an upload answers 401
- **WHEN** Nova picks `a.png`, `b.png` and `c.png`, presses Sign out while the upload of `a.png` is pending, the sign-out request ends without an answer, `a.png` answers 201, and `b.png` answers 401
- **THEN** the screen shows the sign-in form, and no upload request is sent for `c.png`

### Requirement: A late sign-out answer changes nothing

A sign-out answer that arrives after the signed-in view it came from has
ended SHALL change nothing: it SHALL NOT stop any file, change the view or
show a notice. The screen SHALL cancel a pending sign-out request as soon as
the view it came from ends, so that its answer cannot clear the session
cookie of a newer sign-in.

#### Scenario: A pending sign-out is cancelled when its view ends
- **WHEN** Nova presses Sign out, and the events connection says the session ended before the sign-out answers
- **THEN** the screen shows the sign-in form and cancels the sign-out request

#### Scenario: A late sign-out answer leaves the next sign-in alone
- **WHEN** Nova picks `a.png` and `b.png`, presses Sign out while the upload of `a.png` is pending, the events connection says the session ended before the sign-out answers, Nova signs in again and picks `c.png` and `d.png`, the old sign-out answers 204 while the upload of `c.png` is pending, and the uploads of `c.png` and `d.png` both answer 201
- **THEN** the library stays, with no notice, the upload request for `d.png` is sent once `c.png` has its answer, and the lines read `c.png was uploaded.` and `d.png was uploaded.`

#### Scenario: A late failed sign-out shows no notice
- **WHEN** Nova presses Sign out, the events connection says the session ended before the sign-out answers, Nova signs in again, and the old sign-out then answers 500
- **THEN** the library stays and shows no notice
