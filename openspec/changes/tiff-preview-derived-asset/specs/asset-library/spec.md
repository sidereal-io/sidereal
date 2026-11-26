<!-- Plain language: Name the actor, condition and observable result. Explain necessary domain terms using the agreed glossary. Keep requirement and scenario headings and WHEN/THEN syntax exactly as required. -->

## MODIFIED Requirements

### Requirement: An accepted image has a durable Asset and immutable version

A successful first upload SHALL persist an original Asset with an opaque id,
kind, display name and current-version reference, and an Asset version with
its own opaque id, BLAKE3 content hash, byte size, recognized format and
Source provenance. Core SHALL mint identities and compute hashes. The content
hash SHALL NOT be the primary identity or a unique constraint on versions.
Persisted version bytes and their descriptive record SHALL be immutable.

#### Scenario: A first image enters the library
- **WHEN** Nova successfully uploads a PNG named `nebula.png` to an empty library
- **THEN** one original Asset named `nebula.png` points to one persisted version
- **AND** its hash and size describe the original bytes, its format is PNG, and its provenance identifies the manual-upload Source
- **AND** neither opaque id is derived from the filename or content hash

#### Scenario: Restart preserves identity and content
- **WHEN** the server restarts with the same database and paired storage after a successful upload
- **THEN** the same original Asset id, name, current-version id, version hash, size, format and provenance remain available
- **AND** retrieving its original returns the exact uploaded bytes
- **AND** its preview and the preview's lineage remain

### Requirement: The administrator can list library Assets

`GET /api/assets` SHALL return 200 with a JSON `assets` array of committed
originals. Each entry SHALL include `id`, `kind`, `name`,
`current_version_id`, `original_url` and `preview_url`. The `preview_url`
SHALL be `/api/assets/<id>/preview` when the Asset's current version has a
preview, and `null` when it has none. The list SHALL NOT include a derived
Asset, a staging file or an unreferenced stored object. A database failure
SHALL return 503 rather than an empty library.

#### Scenario: Empty library
- **WHEN** a signed-in administrator lists a library with no Assets
- **THEN** the response is 200 with `{"assets":[]}`

#### Scenario: List after upload
- **WHEN** Nova lists the library after uploading `nebula.jpg`, whose preview run succeeded
- **THEN** the response includes its persisted name and identities, a URL for its original bytes and a URL for its preview

#### Scenario: The list holds originals only
- **WHEN** Nova lists the library after uploading `nebula.png` and `m42.tif`, whose preview runs both succeeded
- **THEN** the `assets` array holds exactly two entries, `nebula.png` and `m42.tif`, and no entry for either preview

#### Scenario: An Asset with no preview
- **WHEN** Nova lists the library, which holds `m31.png`, saved before this change
- **THEN** the entry for `m31.png` has `preview_url` `null`

#### Scenario: Database unavailable
- **WHEN** a list request passes the session check but PostgreSQL becomes unavailable before the library query
- **THEN** the response is 503 and does not claim the library is empty

### Requirement: Original retrieval returns stored bytes

`GET /api/assets/{id}/original` SHALL return status 200, the recognized
media type and the bytes stored at the current version's hash path. While the
object is intact, these are the exact uploaded bytes. Retrieval SHALL NOT
re-hash them; finding a damaged or replaced object is left to a later
verify-and-repair feature. Unknown Asset ids, and the ids of derived Assets,
SHALL return 404.

#### Scenario: Original is a JPEG despite its filename
- **WHEN** Nova retrieves an accepted JPEG whose name ends in `.png`
- **THEN** the response body is the uploaded bytes and its Content-Type is `image/jpeg`

#### Scenario: Original of a TIFF
- **WHEN** Nova retrieves the original of an accepted TIFF named `m42.tif`
- **THEN** the response body is the uploaded bytes and its Content-Type is `image/tiff`

#### Scenario: Unknown Asset
- **WHEN** an administrator requests an original for an id not in the library
- **THEN** the response is 404 and contains no file bytes

#### Scenario: A derived Asset's id
- **WHEN** an administrator requests the original for the id of a preview
- **THEN** the response is 404 and contains no file bytes

### Requirement: All library data requires an administrator session

Upload, library listing, reading one Asset, original retrieval and preview
retrieval SHALL use the existing administrator session guard, including its
expiry, revocation and refresh behavior. Upload SHALL also use the existing
allowed-origin check. Neither an original, a preview nor library metadata
SHALL be served through the public web-file fallback.

#### Scenario: No valid session
- **WHEN** a caller with an absent, expired or revoked session requests any Asset endpoint
- **THEN** the response is 401 and contains no library metadata, original bytes or preview bytes
- **AND** an upload creates no Asset and publishes no object

#### Scenario: Forbidden origin
- **WHEN** a signed-in administrator submits an upload from an origin the existing origin check refuses
- **THEN** the response is 403 and the library and object store are unchanged

### Requirement: The administrator can read one Asset

`GET /api/assets/{id}` SHALL return 200 with a JSON body of the form
`{"asset": {…}}`. The `asset` object SHALL hold the same fields as a
library list entry, `preview_url` included, plus `media_type`, the media
type of the Asset's current version, and a `facets` array. A library list
entry and the upload response SHALL NOT hold `media_type`. An id that no
Asset has, the id of a derived Asset, or an id that is not well formed
SHALL return 404. A database failure SHALL return 503.

#### Scenario: The body wraps the Asset
- **WHEN** a signed-in administrator reads the Asset of `nebula.png`
- **THEN** the response is 200, its body has one field, `asset`, and `asset` holds the `id`, `kind`, `name`, `current_version_id`, `original_url` and `preview_url` that the library list gives for that Asset, the `media_type` `image/png`, and a `facets` array

#### Scenario: A JPEG's media type
- **WHEN** a signed-in administrator reads the Asset of `nebula.jpg`
- **THEN** the response is 200 and its `asset.media_type` is `image/jpeg`

#### Scenario: A TIFF's media type
- **WHEN** a signed-in administrator reads the Asset of `m42.tif`, a TIFF
- **THEN** the response is 200 and its `asset.media_type` is `image/tiff`, not the preview's `image/jpeg`

#### Scenario: The bytes decide the media type, not the name
- **WHEN** a signed-in administrator reads the Asset of a file named `m31.jpg` that holds PNG bytes
- **THEN** the response is 200 and its `asset.media_type` is `image/png`

#### Scenario: The list and the upload keep their shape
- **WHEN** a signed-in administrator uploads `nebula.png` and then lists the library
- **THEN** neither the 201 body nor the library list entry for that Asset holds `media_type`

#### Scenario: Unknown or malformed id
- **WHEN** a signed-in administrator reads an id that no Asset has, or the id `not-an-id`
- **THEN** the response is 404 and contains no library data

#### Scenario: A derived Asset's id
- **WHEN** a signed-in administrator reads the id of a preview
- **THEN** the response is 404 and contains no library data

#### Scenario: Database unavailable
- **WHEN** a read passes the session check but PostgreSQL becomes unavailable before the query
- **THEN** the response is 503 and does not claim the Asset is missing or has no facets

### Requirement: An upload of bytes already in the library finds the existing Asset

When the Source accepts an upload whose BLAKE3 content hash equals the hash of
any version of an existing original, and publication stores the bytes or
reuses a stored object that passes its check, ingest SHALL find that original
and commit no new Asset or version. This holds whichever Source brought the
existing version in. When publication fails, the upload SHALL fail as managed storage says, and find no
Asset. An upload the Source refuses SHALL be refused as before, whatever its
hash.

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

### Requirement: An upload finds the oldest of several matching Assets

When versions of several originals have the upload's hash, ingest SHALL find
the original whose matching version was ingested first, and, among versions
ingested at the same moment, the original with the lowest id. All those
Assets SHALL stay as they are.

#### Scenario: Two Assets already share the bytes
- **WHEN** a database built before this change holds two Assets whose versions have the same hash, and Nova uploads those bytes again
- **THEN** the upload finds the Asset whose version was ingested first
- **AND** both Assets and their versions are unchanged, and no third Asset exists

## ADDED Requirements

### Requirement: An upload never finds a derived Asset

The check for a repeat upload SHALL NOT match a derived Asset's version. An
upload whose bytes equal only a preview's SHALL make a new original.

#### Scenario: The bytes of a preview
- **WHEN** the library holds `m42.tif` and its preview, and Nova uploads the preview's bytes as `m42-preview.jpg`
- **THEN** the answer is 201, and the library lists `m42.tif` and a new original named `m42-preview.jpg`
- **AND** the new original's version shares the stored object of the preview's version

### Requirement: Preview retrieval returns the preview's bytes

`GET /api/assets/{id}/preview` SHALL return status 200, the preview's media
type and the bytes stored for the preview of the Asset's current version.
Retrieval SHALL NOT re-hash them. It SHALL return 404 for an unknown or
malformed id, for the id of a derived Asset, and for an original whose
current version has no preview.

#### Scenario: The preview of a TIFF
- **WHEN** Nova retrieves the preview of `m42.tif`, whose preview run succeeded
- **THEN** the response is 200 with Content-Type `image/jpeg`, and its body is the bytes the preview run wrote

#### Scenario: An original with no preview
- **WHEN** Nova retrieves the preview of `m31.png`, saved before this change
- **THEN** the response is 404 and contains no file bytes

#### Scenario: Unknown or derived id
- **WHEN** an administrator retrieves the preview for an id not in the library, or for the id of a preview
- **THEN** the response is 404 and contains no file bytes

### Requirement: A failed preview read never looks complete

When storage is unavailable, or the preview's object is missing, cannot be
opened or is not a regular file, preview retrieval SHALL find this before
the response starts, return a clear 503 and leave every Asset and version
unchanged. When a read fails after the response has started, the server
SHALL abort the response before its end, so that the client never receives
a complete body, and SHALL log the failure.

#### Scenario: The preview's object is missing
- **WHEN** an original has a preview whose stored object is missing
- **THEN** preview retrieval returns 503, and the original, the preview and their versions remain unchanged

#### Scenario: Read fails during the transfer
- **WHEN** reading a preview fails after some of its bytes have been sent
- **THEN** the client sees a failed transfer, not a complete 200 body

### Requirement: Preview responses are private and never sniffed

Every preview response that carries file bytes (status 200) SHALL send
`X-Content-Type-Options: nosniff` and `Cache-Control: no-store`.

#### Scenario: Preview response headers
- **WHEN** Nova retrieves the preview of an accepted TIFF
- **THEN** the response has Content-Type `image/jpeg`, `X-Content-Type-Options: nosniff` and `Cache-Control: no-store`
