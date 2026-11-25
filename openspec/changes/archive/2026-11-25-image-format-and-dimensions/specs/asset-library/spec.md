<!-- Plain language: Name the actor, condition and observable result. Explain necessary domain terms using the agreed glossary. Keep requirement and scenario headings and WHEN/THEN syntax exactly as required. -->

## MODIFIED Requirements

### Requirement: The administrator can read one Asset

`GET /api/assets/{id}` SHALL return 200 with a JSON body of the form
`{"asset": {…}}`. The `asset` object SHALL hold the same fields as a
library list entry, plus `media_type`, the media type of the Asset's current
version, and a `facets` array. A library list entry and the upload response
SHALL NOT hold `media_type`. An id that no Asset has, or that is not a
well-formed id, SHALL return 404. A database failure SHALL return 503.

#### Scenario: The body wraps the Asset
- **WHEN** a signed-in administrator reads the Asset of `nebula.png`
- **THEN** the response is 200, its body has one field, `asset`, and `asset` holds the `id`, `kind`, `name`, `current_version_id` and `original_url` that the library list gives for that Asset, the `media_type` `image/png`, and a `facets` array

#### Scenario: A JPEG's media type
- **WHEN** a signed-in administrator reads the Asset of `nebula.jpg`
- **THEN** the response is 200 and its `asset.media_type` is `image/jpeg`

#### Scenario: The bytes decide the media type, not the name
- **WHEN** a signed-in administrator reads the Asset of a file named `m31.jpg` that holds PNG bytes
- **THEN** the response is 200 and its `asset.media_type` is `image/png`

#### Scenario: The list and the upload keep their shape
- **WHEN** a signed-in administrator uploads `nebula.png` and then lists the library
- **THEN** neither the 201 body nor the library list entry for that Asset holds `media_type`

#### Scenario: Unknown or malformed id
- **WHEN** a signed-in administrator reads an id that no Asset has, or the id `not-an-id`
- **THEN** the response is 404 and contains no library data

#### Scenario: Database unavailable
- **WHEN** a read passes the session check but PostgreSQL becomes unavailable before the query
- **THEN** the response is 503 and does not claim the Asset is missing or has no facets

### Requirement: Reading an Asset returns its facets without provenance

The `facets` array SHALL hold one entry for each facet value of the Asset
and of its current version, ordered by facet name. Each entry SHALL hold
`name`, `label`, `value` and `unit`, with the label and unit taken from the
schema version the value was written under. An Asset with no facet values
SHALL have an empty `facets` array. The response SHALL NOT include
provenance.

#### Scenario: An Asset with its file size
- **WHEN** a signed-in administrator reads the Asset of `nebula.png`, uploaded before this change, with 1,234,567 bytes, whose file-size run succeeded
- **THEN** the response is 200 and its `asset.facets` is `[{"name":"image.file-size","label":"File size","value":1234567,"unit":"bytes"}]`

#### Scenario: An Asset with its file size and dimensions
- **WHEN** a signed-in administrator reads the Asset of `nebula.png`, uploaded after this change, a PNG of 1,234,567 bytes and 4,096 by 2,160 pixels, whose file-size and header runs succeeded
- **THEN** the response is 200 and its `asset.facets` is `[{"name":"image.file-size","label":"File size","value":1234567,"unit":"bytes"},{"name":"image.height","label":"Height","value":2160,"unit":"pixels"},{"name":"image.width","label":"Width","value":4096,"unit":"pixels"}]`

#### Scenario: An Asset whose header run failed
- **WHEN** a signed-in administrator reads the Asset of a PNG of 1,000 bytes, uploaded after this change, whose file-size run succeeded and whose header run failed
- **THEN** the response is 200 and its `asset.facets` is `[{"name":"image.file-size","label":"File size","value":1000,"unit":"bytes"}]`

#### Scenario: An Asset with no facets
- **WHEN** a signed-in administrator reads an Asset that has no facet values
- **THEN** the response is 200 and its `asset.facets` is `[]`
