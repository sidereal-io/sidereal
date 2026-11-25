## RENAMED Requirements

- FROM: `### Requirement: The signed-in library supports one-image upload`
- TO: `### Requirement: The signed-in library uploads the files Nova picks`

- FROM: `### Requirement: After a repeat upload, another file can be chosen at once`
- TO: `### Requirement: A repeat upload does not load the list again`

## MODIFIED Requirements

### Requirement: The signed-in library uploads the files Nova picks

The signed-in view SHALL show one `Upload` button that opens the file picker
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
- **WHEN** an upload request is pending
- **THEN** the view shows the chosen filename as uploading, and neither a second press nor a second choice sends another request

#### Scenario: Library after restart
- **WHEN** Nova opens the signed-in view after the server restarts with the same database and storage
- **THEN** the library loads and displays the saved image by name

#### Scenario: Filename contains markup
- **WHEN** the library contains an image whose display name includes HTML markup
- **THEN** the name appears as literal text and executes no markup

### Requirement: Upload refusals are clear and recoverable

Upload refusals SHALL show a clear message for unsupported format, excessive
size, invalid input or unavailable service, without claiming success. The user
SHALL be able to select other files after a refused upload, once every file
in the same pick has an answer.

#### Scenario: Upload is refused
- **WHEN** Nova picks only `nebula.png`, and its upload returns 413 or 415
- **THEN** Nova sees the relevant limit or supported-format message and can select another file

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

## ADDED Requirements

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
the session, so the pick SHALL go on. A sign-out answer that arrives after
the signed-in view it came from has ended SHALL change nothing: it SHALL NOT
stop any file, change the view or show a notice.

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

#### Scenario: A late sign-out answer leaves the next sign-in alone
- **WHEN** Nova picks `a.png` and `b.png`, presses Sign out while the upload of `a.png` is pending, the events connection says the session ended before the sign-out answers, Nova signs in again and picks `c.png` and `d.png`, the old sign-out answers 204 while the upload of `c.png` is pending, and the uploads of `c.png` and `d.png` both answer 201
- **THEN** the library stays, with no notice, the upload request for `d.png` is sent once `c.png` has its answer, and the lines read `c.png was uploaded.` and `d.png was uploaded.`

#### Scenario: A late failed sign-out shows no notice
- **WHEN** Nova presses Sign out, the events connection says the session ended before the sign-out answers, Nova signs in again, and the old sign-out then answers 500
- **THEN** the library stays and shows no notice
