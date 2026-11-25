## ADDED Requirements

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
