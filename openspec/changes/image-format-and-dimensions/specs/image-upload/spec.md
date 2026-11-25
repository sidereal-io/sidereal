<!-- Plain language: Name the actor, condition and observable result. Explain necessary domain terms using the agreed glossary. Keep requirement and scenario headings and WHEN/THEN syntax exactly as required. -->

## ADDED Requirements

### Requirement: Upload records the image's format and dimensions

After the file-size run ends, and before it answers, the upload SHALL run the
image pack's header Operator over the new version, with the uploading
administrator as the user who started the run. The 201 answer SHALL keep its
existing body. When the header run succeeded, reading the Asset SHALL return
the format, width and height found from the file's bytes, not its name.
Images saved before this change SHALL get no format, width or height.

#### Scenario: A new upload has its format and dimensions
- **WHEN** Nova uploads a PNG of 4,096 by 2,160 pixels, its header run succeeds, and the server answers 201
- **THEN** reading that Asset returns `image.format` with the value `PNG`, `image.width` with the value 4096 and `image.height` with the value 2160, each with the unit its schema gives
- **AND** one operation run of the header Operator exists for its version, with outcome `succeeded`, started by Nova

#### Scenario: The bytes decide the format, not the name
- **WHEN** Nova uploads a file named `m31.jpg` that holds PNG bytes, and the server answers 201
- **THEN** reading that Asset returns `image.format` with the value `PNG`

#### Scenario: An image saved before this change
- **WHEN** the server starts with this change and reads an Asset saved before it, whose only facet is its file size
- **THEN** its `facets` hold only `image.file-size`, and no header run exists for its version

### Requirement: A failed Operator run does not fail the upload

When an upload's file-size or header run fails, its Operator stops
abnormally, or core refuses what it proposed, the upload SHALL still answer
201 with the saved Asset. Core SHALL record the run as failed, with its
reason and none of its values, when it can record the outcome. When it
cannot, the upload SHALL still answer 201, keep the Asset and its version,
and go on to the next run. No run SHALL stop the next run or remove another
run's values, and the server SHALL go on answering requests.

#### Scenario: The header run fails
- **WHEN** Nova uploads a file of 1,000 bytes that starts with the JPEG signature but has no readable frame header
- **THEN** the response is 201, the Library lists the image, and reading the Asset returns only the facet `image.file-size`, with the value 1000
- **AND** the header run is `failed` with the reason `the JPEG header could not be read`

#### Scenario: The file-size run fails
- **WHEN** Nova uploads a PNG and the file-size run fails
- **THEN** the response is 201, the Library lists the image, and reading the Asset returns no `image.file-size` facet
- **AND** the file-size run is `failed` with a reason, and a header run is still recorded for its version

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

## MODIFIED Requirements

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
its connection SHALL NOT stop the Asset's save, its file-size run or its
header run. Each run SHALL end as it would have with the client connected.

#### Scenario: The client goes away while the Asset is being saved
- **WHEN** the client closes its connection after the upload has passed its last cancellation check and before the Asset's transaction commits
- **THEN** the Asset is saved, one file-size run and one header run are recorded for its version, each with outcome `succeeded`, and reading the Asset afterwards returns its file size, format, width and height

#### Scenario: The client goes away during the run
- **WHEN** the client closes its connection after the Asset is saved and before its upload runs have ended
- **THEN** the file-size run and the header run both end `succeeded`, as they would have with the client connected, and reading the Asset afterwards returns its file size, format, width and height

## REMOVED Requirements

### Requirement: A failed file-size run does not fail the upload
**Reason**: Upload now runs two Operators. "A failed Operator run does not fail the upload" and "The server log names each failed upload run" cover both runs and keep every rule this requirement held.
**Migration**: The server log lines now start with the Operator's id instead of `file-size`: read `image.measure-file-size run failed for asset <id>: <reason>` in place of `file-size run failed for asset <id>: <reason>`, and `image.measure-file-size run not recorded for asset <id>:` in place of `file-size run not recorded for asset <id>:`.
