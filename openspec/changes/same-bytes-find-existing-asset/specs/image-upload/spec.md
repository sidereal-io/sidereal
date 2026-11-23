## ADDED Requirements

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
