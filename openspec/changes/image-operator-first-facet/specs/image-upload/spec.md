<!-- Plain language: Name the actor, condition and observable result. Explain necessary domain terms using the agreed glossary. Keep requirement and scenario headings and WHEN/THEN syntax exactly as required. -->

## ADDED Requirements

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
