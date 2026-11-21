## ADDED Requirements

### Requirement: Ingest publishes durable objects before library records

Ingest SHALL copy candidate bytes into a unique `tmp/{uuid}` within the paired
storage root, compute their BLAKE3 hash and size, and have the Source recognize
the format. It SHALL sync the complete file, atomically publish it at
`store/{h[0:2]}/{h[2:4]}/{h}` where `h` is the lowercase hexadecimal hash, and
sync the containing directory and newly created ancestor entries before
committing any Asset or version. Staging and store SHALL be on the same
filesystem. Failure of any required write, sync or publication SHALL prevent
the database commit. Startup SHALL still leave a newly paired empty root with
only its marker; ingest creates its directories when first needed.

#### Scenario: First successful ingest
- **WHEN** Nova uploads a new recognized image into paired storage
- **THEN** its original exists at the BLAKE3-derived path with identical bytes before the Asset commit
- **AND** file and directory sync have completed before that commit

#### Scenario: Publication fails
- **WHEN** a write, sync or atomic publication fails during ingest
- **THEN** no Asset or version for the upload is committed and existing stored objects remain unchanged

#### Scenario: Database commit fails after publication
- **WHEN** object publication succeeds but the Asset transaction fails
- **THEN** the library contains no partial Asset or version for that upload
- **AND** the published object remains available for later reuse and is not reclaimed

### Requirement: Matching content shares one stored object

When a regular object already exists at the candidate's hash path, ingest
SHALL reuse that object without overwriting it. Concurrent ingests of matching
bytes SHALL retain one complete object at that path. This requirement concerns
byte storage only; finding an existing logical Asset is deferred to the
Asset-deduplication change. Stored objects SHALL NOT be reclaimed by this slice.

#### Scenario: Matching bytes arrive again
- **WHEN** another accepted upload has the hash of an existing object
- **THEN** ingest reuses that object without changing it or creating another stored copy

#### Scenario: Concurrent matching bytes
- **WHEN** two requests concurrently publish the same candidate bytes
- **THEN** each successful commit refers to the same complete stored object
- **AND** neither request truncates or replaces the other's object

### Requirement: Ingest and reads stay inside managed storage

Object paths SHALL derive from core-validated hashes and staging paths from
core-generated identifiers, never from a display filename or caller path.
Storage operations SHALL reject path traversal, symlink escape and non-regular
object or staging files, including a symlink in an intermediate directory.
A refused operation SHALL NOT alter or disclose files outside managed storage.

#### Scenario: Filename contains traversal
- **WHEN** an upload supplies a name such as `../../outside.png`
- **THEN** no part of that name selects a storage location and no file outside the paired root is read or written

#### Scenario: Store directory is a symlink
- **WHEN** `store`, a hash directory or the final object is replaced by a symlink to a path outside the root
- **THEN** ingest or original retrieval refuses that path without reading or modifying the target

#### Scenario: Staging directory is unsafe
- **WHEN** `tmp` is a symlink or the proposed staging entry is already present or a special file
- **THEN** ingest does not follow or overwrite that entry and commits no Asset through it
