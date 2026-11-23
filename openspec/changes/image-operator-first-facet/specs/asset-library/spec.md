<!-- Plain language: Name the actor, condition and observable result. Explain necessary domain terms using the agreed glossary. Keep requirement and scenario headings and WHEN/THEN syntax exactly as required. -->

## ADDED Requirements

### Requirement: The administrator can read one Asset with its facets

`GET /api/assets/{id}` SHALL return 200 with a JSON `asset` object. It SHALL
hold the same fields as a library list entry, plus a `facets` array. The
array SHALL hold one entry for each facet value of the Asset and of its
current version, ordered by facet name. Each entry SHALL hold `name`,
`label`, `value` and `unit`, with the label and unit taken from the facet's
schema. An Asset with no facet values SHALL have an empty `facets` array. The
response SHALL NOT include provenance. An id that no Asset has, or that is
not a well-formed id, SHALL return 404. A database failure SHALL return 503.

#### Scenario: An Asset with its file size
- **WHEN** a signed-in administrator reads the Asset of `nebula.png`, uploaded after this change, with 1,234,567 bytes
- **THEN** the response is 200 and its `asset.facets` is `[{"name":"image.file-size","label":"File size","value":1234567,"unit":"bytes"}]`

#### Scenario: An Asset with no facets
- **WHEN** a signed-in administrator reads an Asset that has no facet values
- **THEN** the response is 200 and its `asset.facets` is `[]`

#### Scenario: Unknown or malformed id
- **WHEN** a signed-in administrator reads an id that no Asset has, or the id `not-an-id`
- **THEN** the response is 404 and contains no library data

#### Scenario: Database unavailable
- **WHEN** a read passes the session check but PostgreSQL becomes unavailable before the query
- **THEN** the response is 503 and does not claim the Asset is missing or has no facets

## MODIFIED Requirements

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
