<!-- Plain language: Name the actor, condition and observable result. Explain necessary domain terms using the agreed glossary. Keep requirement and scenario headings and WHEN/THEN syntax exactly as required. -->

## ADDED Requirements

### Requirement: Upload records the image's file size

After a successful upload saves the Asset, and before it answers, the server
SHALL run the image pack's file-size Operator over the new version. The
upload's 201 answer SHALL keep its existing body. Once the client has the
answer, reading the Asset SHALL return its file size. A client that goes away
after the Asset is saved SHALL NOT stop the run. Images saved before this
change SHALL get no file size.

#### Scenario: A new upload has its file size
- **WHEN** Nova uploads a PNG of 1,234,567 bytes and the server answers 201
- **THEN** reading that Asset returns the facet `image.file-size` with the value 1234567 and the unit `bytes`
- **AND** one operation run of the file-size Operator exists for its version, with outcome `succeeded`

#### Scenario: The client goes away during the run
- **WHEN** the client closes its connection after the Asset is saved and while the file-size Operator is running
- **THEN** the run ends as it would have, and reading the Asset afterwards returns its file size

### Requirement: A failed file-size run does not fail the upload

When the file-size run fails, when core refuses what it proposed, or when the
server cannot record the run, the upload SHALL still answer 201 with the
saved Asset. The Asset SHALL have no file size. The server log SHALL name the
failure. A run that core could record SHALL be recorded as failed with its
reason.

#### Scenario: The run fails
- **WHEN** Nova uploads a PNG and the file-size run fails
- **THEN** the response is 201, the Library lists the image, and reading the Asset returns `"facets":[]`
- **AND** its operation run is `failed` with a reason

#### Scenario: The run cannot be recorded
- **WHEN** Nova uploads a PNG and PostgreSQL refuses the run's record after the Asset is saved
- **THEN** the response is 201 and the Asset and its version remain
- **AND** the server log names the failure
