# asset-library Specification

## Purpose
Let Nova find uploaded images by stable identity and retrieve their original
bytes after the server restarts. Assets and Asset versions use the shared glossary.

## Requirements

### Requirement: An accepted image has a durable Asset and immutable version

A successful first upload SHALL persist an Asset with an opaque id, kind,
display name and current-version reference, and an Asset version with its own
opaque id, BLAKE3 content hash, byte size, recognized format and Source
provenance. Core SHALL mint identities and compute hashes. The content hash
SHALL NOT be the primary identity or a unique constraint on versions.
Persisted version bytes and their descriptive record SHALL be immutable.

#### Scenario: A first image enters the library
- **WHEN** Nova successfully uploads a PNG named `nebula.png` to an empty library
- **THEN** one Asset named `nebula.png` points to one persisted version
- **AND** its hash and size describe the original bytes, its format is PNG, and its provenance identifies the manual-upload Source
- **AND** neither opaque id is derived from the filename or content hash

#### Scenario: Restart preserves identity and content
- **WHEN** the server restarts with the same database and paired storage after a successful upload
- **THEN** the same Asset id, name, current-version id, version hash, size, format and provenance remain available
- **AND** retrieving its original returns the exact uploaded bytes

### Requirement: The administrator can list library Assets

`GET /api/assets` SHALL return 200 with a JSON `assets` array of committed
Assets. Each entry SHALL include `id`, `kind`, `name`, `current_version_id`
and `original_url`. It SHALL NOT list staging files or unreferenced stored
objects. A database failure SHALL return 503 rather than an empty library.

#### Scenario: Empty library
- **WHEN** a signed-in administrator lists a library with no Assets
- **THEN** the response is 200 with `{"assets":[]}`

#### Scenario: List after upload
- **WHEN** Nova lists the library after uploading `nebula.jpg`
- **THEN** the response includes its persisted name and identities and a URL for its original bytes

#### Scenario: Database unavailable
- **WHEN** a list request passes the session check but PostgreSQL becomes unavailable before the library query
- **THEN** the response is 503 and does not claim the library is empty

### Requirement: Original retrieval returns stored bytes

`GET /api/assets/{id}/original` SHALL return status 200, the recognized
media type and the bytes stored at the current version's hash path. While the
object is intact, these are the exact uploaded bytes. Retrieval SHALL NOT
re-hash them; finding a damaged or replaced object is left to a later
verify-and-repair feature. Unknown Asset ids SHALL return 404.

#### Scenario: Original is a JPEG despite its filename
- **WHEN** Nova retrieves an accepted JPEG whose name ends in `.png`
- **THEN** the response body is the uploaded bytes and its Content-Type is `image/jpeg`

#### Scenario: Unknown Asset
- **WHEN** an administrator requests an original for an id not in the library
- **THEN** the response is 404 and contains no file bytes

### Requirement: A failed original read never looks complete

When storage is unavailable, or the object is missing, cannot be opened or is
not a regular file, retrieval SHALL find this before the response starts,
return a clear 503 and leave the Asset and its version unchanged. When a read
fails after the response has started, the server SHALL abort the response
before its end, so that the client never receives a complete body, and SHALL
log the failure.

#### Scenario: Original is missing
- **WHEN** an Asset exists but its original object is missing, cannot be opened or is not a regular file
- **THEN** original retrieval returns 503 and the Asset and version remain unchanged

#### Scenario: Read fails during the transfer
- **WHEN** reading an original fails after some of its bytes have been sent
- **THEN** the client sees a failed transfer, not a complete 200 body
- **AND** the Asset and version remain unchanged

### Requirement: Original responses are private and never sniffed

Every original response that carries file bytes (status 200) SHALL send
`X-Content-Type-Options: nosniff` and `Cache-Control: no-store`. The session
guard's 401 responses carry no file bytes and keep their existing headers.

#### Scenario: Original response headers
- **WHEN** Nova retrieves the original of an accepted PNG
- **THEN** the response has Content-Type `image/png`, `X-Content-Type-Options: nosniff` and `Cache-Control: no-store`

### Requirement: All library data requires an administrator session

Upload, library listing, reading one Asset and original retrieval SHALL use
the existing administrator session guard, including its expiry, revocation
and refresh behavior. Upload SHALL also use the existing allowed-origin
check. Neither an original nor library metadata SHALL be served through the
public web-file fallback.

#### Scenario: No valid session
- **WHEN** a caller with an absent, expired or revoked session requests any Asset endpoint
- **THEN** the response is 401 and contains no library metadata or original bytes
- **AND** an upload creates no Asset and publishes no object

#### Scenario: Forbidden origin
- **WHEN** a signed-in administrator submits an upload from an origin the existing origin check refuses
- **THEN** the response is 403 and the library and object store are unchanged

### Requirement: The administrator can read one Asset

`GET /api/assets/{id}` SHALL return 200 with a JSON body of the form
`{"asset": {…}}`. The `asset` object SHALL hold the same fields as a
library list entry, plus a `facets` array. An id that no Asset has, or that
is not a well-formed id, SHALL return 404. A database failure SHALL return
503.

#### Scenario: The body wraps the Asset
- **WHEN** a signed-in administrator reads the Asset of `nebula.png`
- **THEN** the response is 200, its body has one field, `asset`, and `asset` holds the `id`, `kind`, `name`, `current_version_id` and `original_url` that the library list gives for that Asset, and a `facets` array

#### Scenario: Unknown or malformed id
- **WHEN** a signed-in administrator reads an id that no Asset has, or the id `not-an-id`
- **THEN** the response is 404 and contains no library data

#### Scenario: Database unavailable
- **WHEN** a read passes the session check but PostgreSQL becomes unavailable before the query
- **THEN** the response is 503 and does not claim the Asset is missing or has no facets

### Requirement: Reading an Asset returns its facets without provenance

The `facets` array SHALL hold one entry for each facet value of the Asset
and of its current version, ordered by facet name. Each entry SHALL hold
`name`, `label`, `value` and `unit`, with the label and unit of the schema
version the value was written under. A count's `value` SHALL be a JSON
number and a choice's a JSON string. A facet with no unit SHALL have the
`unit` `""`. An Asset with no facet values SHALL have an empty `facets`
array. The response SHALL NOT include provenance.

#### Scenario: An Asset with its file size
- **WHEN** a signed-in administrator reads the Asset of `nebula.png`, uploaded before this change, with 1,234,567 bytes, whose file-size run succeeded
- **THEN** the response is 200 and its `asset.facets` is `[{"name":"image.file-size","label":"File size","value":1234567,"unit":"bytes"}]`

#### Scenario: An Asset with its file size, format and dimensions
- **WHEN** a signed-in administrator reads the Asset of `nebula.png`, uploaded after this change, a PNG of 1,234,567 bytes and 4,096 by 2,160 pixels, whose file-size and header runs succeeded
- **THEN** the response is 200 and its `asset.facets` is `[{"name":"image.file-size","label":"File size","value":1234567,"unit":"bytes"},{"name":"image.format","label":"Format","value":"PNG","unit":""},{"name":"image.height","label":"Height","value":2160,"unit":"pixels"},{"name":"image.width","label":"Width","value":4096,"unit":"pixels"}]`

#### Scenario: An Asset whose header run failed
- **WHEN** a signed-in administrator reads the Asset of a PNG of 1,000 bytes, uploaded after this change, whose file-size run succeeded and whose header run failed
- **THEN** the response is 200 and its `asset.facets` is `[{"name":"image.file-size","label":"File size","value":1000,"unit":"bytes"}]`

#### Scenario: An Asset with no facets
- **WHEN** a signed-in administrator reads an Asset that has no facet values
- **THEN** the response is 200 and its `asset.facets` is `[]`

### Requirement: An upload of bytes already in the library finds the existing Asset

When the Source accepts an upload whose BLAKE3 content hash equals the hash of
any version of an existing Asset, and publication stores the bytes or reuses
a stored object that passes its check, ingest SHALL find that Asset and
commit no new Asset or version. This holds whichever Source brought the
existing version in. When publication fails, the upload SHALL fail as managed
storage says, and find no Asset. An upload the Source refuses SHALL be
refused as before, whatever its hash.

#### Scenario: Different bytes make a new Asset
- **WHEN** the library holds `nebula.png`, and Nova uploads `nebula.jpg`, whose bytes differ
- **THEN** the library holds two Assets

#### Scenario: Another Source brought the bytes in
- **WHEN** the library holds an Asset whose version has the upload's hash and another Source's provenance, and the Source accepts the upload
- **THEN** the upload finds that Asset
- **AND** its version keeps the other Source's provenance, and no new Asset or version exists

#### Scenario: The Source refuses bytes already in the library
- **WHEN** the library holds an Asset whose version has the upload's hash, and the Source refuses the upload
- **THEN** the upload is refused
- **AND** the Assets and versions are unchanged

### Requirement: The found Asset stays as it is

The Asset an upload finds SHALL keep its name, kind, current version and
provenance. Ingest SHALL NOT store the upload's filename anywhere.

#### Scenario: The same bytes under another name
- **WHEN** the library holds one Asset named `nebula.png`, and Nova uploads the same bytes as `m31-final.png`
- **THEN** the library still holds exactly one Asset and one Asset version
- **AND** that Asset is still named `nebula.png`, with the same id, current-version id and provenance as before

### Requirement: An upload finds the oldest of several matching Assets

When versions of several Assets have the upload's hash, ingest SHALL find the
Asset whose matching version was ingested first, and, among versions ingested
at the same moment, the Asset with the lowest id. All those Assets SHALL stay
as they are.

#### Scenario: Two Assets already share the bytes
- **WHEN** a database built before this change holds two Assets whose versions have the same hash, and Nova uploads those bytes again
- **THEN** the upload finds the Asset whose version was ingested first
- **AND** both Assets and their versions are unchanged, and no third Asset exists

### Requirement: Concurrent uploads of the same bytes make one Asset

When two or more uploads of the same bytes, none already in the library,
reach the Asset commit at the same time, exactly one SHALL commit a new Asset.
Every other one SHALL find that Asset, as an upload of bytes already in the
library does.

#### Scenario: Two uploads at the same moment
- **WHEN** two uploads of the same new bytes, named `a.png` and `b.png`, reach the Asset commit together
- **THEN** the library holds exactly one Asset and one Asset version for those bytes
- **AND** one upload reports a new Asset and the other reports that same Asset as found
