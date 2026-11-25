## RENAMED Requirements

- FROM: `### Requirement: The signed-in library supports one-image upload`
- TO: `### Requirement: The signed-in library uploads the files Nova picks`

- FROM: `### Requirement: After a repeat upload, another file can be chosen at once`
- TO: `### Requirement: A repeat upload does not load the list again`

## MODIFIED Requirements

### Requirement: The signed-in library uploads the files Nova picks

The signed-in view SHALL show one `Upload` button that opens the file picker
for one or more PNG or JPEG files. Choosing files SHALL start their upload at
once. The view SHALL send the chosen files one at a time, in the order
picked, as one upload request each. It SHALL send a file only after the file
before it has an answer or has ended without one. Each committed image SHALL
show with its name, as text, and its original. Each saved file SHALL refresh
the library without a page reload. Health, username, sign-out and
event-socket behavior SHALL stay as they are, and the view SHALL follow
DESIGN.md.

#### Scenario: First upload appears
- **WHEN** Nova presses Upload, chooses `nebula.png`, and the server accepts it
- **THEN** the library shows `nebula.png` and its image without reloading

#### Scenario: Files go in turn
- **WHEN** Nova picks `a.png`, `b.png` and `c.png` in one pick, and the server answers each upload only when the test says so
- **THEN** exactly one upload request is pending at any time
- **AND** the requests carry `a.png`, then `b.png`, then `c.png`, each as the only file in its request

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
- **WHEN** Nova picks `a.png`, `b.png` and `c.png`, and the server answers 201, then 409, then 201
- **THEN** the list is requested three times in all: once when the library opens, once after `a.png` and once after `c.png`

## ADDED Requirements

### Requirement: Each chosen file has a line that shows its progress

The library screen SHALL show one line for each file in the current pick, in
the order picked. A file not yet sent SHALL read `<name> is waiting.` A file
on its way SHALL read `Uploading <name>…`. A saved file SHALL read
`<name> was uploaded.` A refused file, a file already in the library, and a
file whose upload ended without an answer SHALL read the messages that
"Upload refusals are clear and recoverable", "The library screen says when
an image is already there" and "A lost upload answer says another upload is
safe" define. Each line SHALL stay until Nova next picks files. The next pick
SHALL replace every line with the lines of its own files. A pick of one file
SHALL follow the same rules. Every name SHALL appear as text only.

#### Scenario: Lines while the second file uploads
- **WHEN** Nova picks `a.png`, `b.png` and `c.png`, the server accepts `a.png`, and the upload of `b.png` is pending
- **THEN** the screen shows `a.png was uploaded.`, `Uploading b.png…` and `c.png is waiting.`, in that order

#### Scenario: One file's line stays after its answer
- **WHEN** Nova picks only `nebula.png`, and the server accepts it
- **THEN** the screen shows `nebula.png was uploaded.` and the image `nebula.png`

#### Scenario: The next pick replaces the lines
- **WHEN** every file in Nova's pick of `a.png` and `b.png` has an answer, and Nova then picks `c.png`
- **THEN** the screen shows a line for `c.png` and no line for `a.png` or `b.png`

#### Scenario: A waiting filename contains markup
- **WHEN** Nova picks `a.png` and `<b>m42</b>.png`, and the upload of `a.png` is pending
- **THEN** the screen shows `<b>m42</b>.png is waiting.` as literal text and no `b` element

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

When the session ends while files in a pick are still waiting, the library
screen SHALL NOT send them. The library and every file's line SHALL be
hidden, as "Ending the session hides library data" requires.

#### Scenario: An upload answers 401
- **WHEN** Nova picks `a.png`, `b.png` and `c.png`, and the upload of `a.png` answers 401
- **THEN** the screen reports the lost session, and no upload request is sent for `b.png` or `c.png`

#### Scenario: Nova signs out during a pick
- **WHEN** Nova picks `a.png` and `b.png`, and signs out while the upload of `a.png` is pending
- **THEN** the library and the lines disappear, no upload request is sent for `b.png`, and a later answer for `a.png` shows nothing
