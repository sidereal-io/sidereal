## Purpose

Let Nova find uploaded images by stable identity and retrieve their original
bytes after the server restarts. Assets and Asset versions use the shared glossary.

## ADDED Requirements

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

Upload, library listing and original retrieval SHALL use the existing
administrator session guard, including its expiry, revocation and refresh
behavior. Upload SHALL also use the existing allowed-origin check. Neither
an original nor library metadata SHALL be served through the public web-file
fallback.

#### Scenario: No valid session
- **WHEN** a caller with an absent, expired or revoked session requests any Asset endpoint
- **THEN** the response is 401 and contains no library metadata or original bytes
- **AND** an upload creates no Asset and publishes no object

#### Scenario: Forbidden origin
- **WHEN** a signed-in administrator submits an upload from an origin the existing origin check refuses
- **THEN** the response is 403 and the library and object store are unchanged
