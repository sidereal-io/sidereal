# image-upload Specification

## Purpose
Let Nova bring one PNG or JPEG from their computer into the library through
the public Source contract and see the resulting image while signed in.

## Requirements

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

The manual-upload Source SHALL accept recognized PNG, JPEG or TIFF bytes
regardless of filename extension or client media type. A TIFF is recognized
by the first four bytes of a classic TIFF or a BigTIFF, in either byte
order. Its media type SHALL be `image/tiff`. The Source SHALL refuse an
unrecognized format, before any Asset is created, with the reason
`only PNG, JPEG and TIFF images are supported`, and the upload SHALL answer
415. Recognition SHALL NOT require full image decoding.

#### Scenario: Misleading extension
- **WHEN** Nova uploads recognized PNG bytes with filename `nebula.txt` and client type `text/plain`
- **THEN** the upload is accepted with PNG format

#### Scenario: A TIFF in each layout
- **WHEN** Nova uploads four files that start with `II*` and a zero byte, `MM`, a zero byte and `*`, `II+` and a zero byte, and `MM`, a zero byte and `+`
- **THEN** each upload is accepted, and each Asset's original has the media type `image/tiff`

#### Scenario: Unsupported bytes
- **WHEN** Nova uploads GIF, SVG, empty bytes or plain text named `nebula.tif`
- **THEN** the response is 415 with the `error` `unsupported format: only PNG, JPEG and TIFF images are supported` and the `reason` `only PNG, JPEG and TIFF images are supported`, and creates no Asset

#### Scenario: Recognizable but damaged image
- **WHEN** a candidate has a recognized PNG, JPEG or TIFF signature but cannot fully decode
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
for one or more files of any type, because the server decides which formats
it takes. Choosing files SHALL start their
upload at once, as "The chosen files go one at a time" says. Each committed
image SHALL show with its name, as text, and its image. Each saved file SHALL refresh
the library without a page reload. Health, username, sign-out and
event-socket behavior SHALL stay as they are, and the view SHALL follow
DESIGN.md.

#### Scenario: First upload appears
- **WHEN** Nova presses Upload, chooses `nebula.png`, and the server accepts it
- **THEN** the library shows `nebula.png` and its image without reloading

#### Scenario: Several new images appear
- **WHEN** Nova picks `nebula.png` and `m31.jpg` in one pick, and the server accepts both
- **THEN** the library shows both images by name without reloading

#### Scenario: The picker offers any file
- **WHEN** Nova presses Upload
- **THEN** the file picker limits the files it offers to no type

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
after the 201 SHALL return its file size; when it did not, "A failed
Operator run does not fail the upload" applies. Images saved before Sidereal
recorded file sizes SHALL get no file size.

#### Scenario: A new upload has its file size
- **WHEN** Nova uploads a PNG of 1,234,567 bytes, its file-size run succeeds, and the server answers 201
- **THEN** reading that Asset returns the facet `image.file-size` with the value 1234567 and the unit `bytes`
- **AND** one operation run of the file-size Operator exists for its version, with outcome `succeeded`, started by Nova

### Requirement: A client that goes away does not stop the save or the run

Once the upload has passed its last cancellation check, a client that closes
its connection SHALL NOT stop the Asset's save, its file-size run, its
header run or its preview run. Each run SHALL end as it would have with the
client connected.

#### Scenario: The client goes away while the Asset is being saved
- **WHEN** the client closes its connection after the upload has passed its last cancellation check and before the Asset's transaction commits
- **THEN** the Asset is saved, one file-size run, one header run and one preview run are recorded for its version, each with outcome `succeeded`, and reading the Asset afterwards returns its file size, width and height and a `preview_url`

#### Scenario: The client goes away during the run
- **WHEN** the client closes its connection after the Asset is saved and before its upload runs have ended
- **THEN** the file-size run, the header run and the preview run all end `succeeded`, as they would have with the client connected, and reading the Asset afterwards returns its file size, width and height and a `preview_url`

### Requirement: A repeat upload answers 409 with the existing Asset

When ingest finds the existing Asset for an upload, as the asset-library
capability defines, `POST /api/assets` SHALL answer 409 Conflict with a JSON
body holding an `asset` object for the found Asset. The object SHALL use the
same entry fields as the library list and the 201 answer: `id`, `kind`,
`name`, `current_version_id`, `original_url` and `preview_url`. A first upload
SHALL still answer 201. A failed publication SHALL still answer 503, not 409.

#### Scenario: The same PNG again
- **WHEN** Nova uploads `nebula.png`, and then uploads the same bytes as `m31-final.png`
- **THEN** the first answer is 201 and the second is 409
- **AND** the 409 body's `asset` has the first answer's `id`, `current_version_id`, `original_url` and `preview_url`, and `name` `nebula.png`

#### Scenario: A repeat of an image with no preview
- **WHEN** Nova uploads a CMYK TIFF whose preview run fails, and then uploads the same bytes again
- **THEN** the second answer is 409, and its `asset.preview_url` is `null`

#### Scenario: A refused upload is not a repeat
- **WHEN** Nova uploads plain text named `nebula.png`
- **THEN** the answer is 415, and not 409

#### Scenario: The stored object of the existing Asset is damaged
- **WHEN** Nova uploads `nebula.png`, its stored object's bytes then change, and Nova uploads the same bytes again
- **THEN** the second answer is 503, and not 409
- **AND** the library still holds one original and its one version, and the original's stored object is unchanged

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
`<reason>` SHALL be `it is larger than the 100 MiB limit.` for 413, the
reason that "A refused format's line shows the server's reason" gives for
415, `choose one image file and try again.` for 400, and
`the server could not save it. Try again later.` for any other refusal. The
web SHALL name no format of its own.

#### Scenario: Each kind of refusal
- **WHEN** Nova picks `a.png`, `b.png`, `c.png` and `d.png`, and the server answers 413, 415 with the `reason` `only PNG, JPEG and TIFF images are supported`, 400 and 503
- **THEN** the lines read `a.png was not uploaded: it is larger than the 100 MiB limit.`, `b.png was not uploaded: only PNG, JPEG and TIFF images are supported.`, `c.png was not uploaded: choose one image file and try again.` and `d.png was not uploaded: the server could not save it. Try again later.`, in that order

### Requirement: One file's answer does not stop the others

A refusal, a 409, or an upload that ends without an answer SHALL NOT stop the
upload of the files after it in the same pick. Each of those files SHALL be
sent as usual.

#### Scenario: Every kind of answer in one pick
- **WHEN** Nova picks `a.png`, `b.png`, `c.png` and `d.png`, and the server answers 415 with the `reason` `only PNG, JPEG and TIFF images are supported` for `a.png`, 409 for `b.png`, nothing for `c.png` and 201 for `d.png`
- **THEN** four upload requests are sent, one for each file, in the order picked
- **AND** the screen shows `a.png was not uploaded: only PNG, JPEG and TIFF images are supported.`, `b.png is already in the library.`, `No answer came back for c.png. It is safe to upload it again: if it was saved, Sidereal will find it.` and `d.png was uploaded.`, in that order

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

### Requirement: Upload records the image's dimensions

After the file-size run ends, and before it answers, the upload SHALL run the
image pack's header Operator over the new version, with the uploading
administrator as the user who started the run. The 201 answer SHALL keep its
existing body. When the header run succeeded, reading the Asset SHALL return
the width and height from the file's header, read by the format that the
file's bytes show, not its name. Images saved before this change SHALL get
no width or height.

#### Scenario: A new upload has its dimensions
- **WHEN** Nova uploads a PNG of 4,096 by 2,160 pixels, its header run succeeds, and the server answers 201
- **THEN** reading that Asset returns `image.width` with the value 4096 and `image.height` with the value 2160, each with the unit `pixels`
- **AND** one operation run of the header Operator exists for its version, with outcome `succeeded`, started by Nova

#### Scenario: The bytes decide the format, not the name
- **WHEN** Nova uploads a file named `m31.jpg` that holds the PNG bytes of a 32 by 24 pixel image, and the server answers 201
- **THEN** reading that Asset returns the `media_type` `image/png`, `image.width` with the value 32 and `image.height` with the value 24

#### Scenario: An image saved before this change
- **WHEN** the server starts with this change and reads an Asset saved before it, whose only facet is its file size
- **THEN** its `facets` hold only `image.file-size`, and no header run exists for its version

### Requirement: A failed Operator run does not fail the upload

When an upload's file-size, header or preview run fails, its Operator stops
abnormally, or core refuses what it proposed, the upload SHALL still answer
201 with the saved Asset. Core SHALL record the run failed, with its reason
and none of its values or Assets, when it can.
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
- **THEN** the response is 201, its `asset.preview_url` is `null`, and reading the Asset returns the `media_type` `image/tiff`, `image.width` 800 and `image.height` 600
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
- **THEN** the response is 201, the Asset and its version remain, and reading the Asset returns no `image.width` or `image.height` facet
- **AND** the server log has a line that starts `image.read-header run not recorded for asset <id>:`

### Requirement: The server log names each failed upload run

For each upload run that fails, the server log SHALL have the line
`<operator id> run failed for asset <id>: <reason>`. For each upload run
whose outcome the server could not record, the server log SHALL have a line
that starts `<operator id> run not recorded for asset <id>:`.

#### Scenario: The header run fails
- **WHEN** Nova uploads a file of 1,000 bytes that starts with the JPEG signature but has no readable frame header
- **THEN** the server log has the line `image.read-header run failed for asset <id>: the JPEG header could not be read`

#### Scenario: The file-size run fails
- **WHEN** Nova uploads a PNG and the file-size run fails with a reason
- **THEN** the server log has the line `image.measure-file-size run failed for asset <id>: <reason>`

#### Scenario: The runs cannot be recorded
- **WHEN** Nova uploads a PNG and PostgreSQL refuses the runs' records after the Asset is saved
- **THEN** the server log has a line that starts `image.measure-file-size run not recorded for asset <id>:` and a line that starts `image.read-header run not recorded for asset <id>:`

### Requirement: A refused upload carries the Source's reason

When the Source refuses a file, the upload's 415 answer SHALL be a JSON
body whose `reason` is the Source's own reason, and whose `error` is
`unsupported format: ` followed by that reason. The server SHALL add no
list of formats of its own.

#### Scenario: The refusal is the Source's
- **WHEN** a host's manual-upload Source refuses every file with the reason `only FITS files are supported`, and Nova uploads a PNG through it
- **THEN** the response is 415 with the `error` `unsupported format: only FITS files are supported` and the `reason` `only FITS files are supported`, and creates no Asset

### Requirement: A refused format's line shows the server's reason

For a 415, a refused file's reason SHALL be the answer's `reason`,
followed by a full stop unless it ends with one, when that `reason` is text
of 1 to 200 characters with no control character. Otherwise it SHALL be
`its format is not supported.` The line SHALL show the server's reason as
literal text.

#### Scenario: A 415 with no usable reason
- **WHEN** Nova picks `a.gif`, `b.gif` and `c.gif`, and the server answers 415 with no `reason`, with a `reason` of 201 characters, and with a `reason` that holds a line break
- **THEN** each line reads `<name> was not uploaded: its format is not supported.`

#### Scenario: A reason holds markup
- **WHEN** the server answers 415 with a `reason` that includes HTML markup
- **THEN** the line shows the reason as literal text and runs no markup

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

### Requirement: An empty library asks for an image

A library with no Assets SHALL show the line
`No images yet. Upload an image to start.` It SHALL name no format.

#### Scenario: An empty library
- **WHEN** Nova opens the signed-in view of a library with no Assets
- **THEN** the Library shows `No images yet. Upload an image to start.`

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

#### Scenario: A preview arrives after the original failed to show
- **WHEN** the Library lists `m42.tif` with `preview_url` `null`, its original fails to show, and a later list gives it `/api/assets/<id>/preview`
- **THEN** the Library shows its image from `/api/assets/<id>/preview`

#### Scenario: A TIFF whose preview run failed
- **WHEN** the library holds `cmyk.tif`, whose preview run failed, and the browser cannot show its original
- **THEN** the Library shows the name `cmyk.tif` beside `This image could not be displayed.`

#### Scenario: A preview address from elsewhere
- **WHEN** the list gives an Asset a `preview_url` other than `/api/assets/<id>/preview` for that Asset's own id, or a value that is neither text nor `null`
- **THEN** the Library treats the list as not valid, and shows its library error and Retry control
