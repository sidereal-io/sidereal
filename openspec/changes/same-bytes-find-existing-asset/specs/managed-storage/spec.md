## MODIFIED Requirements

### Requirement: Matching content shares one stored object

When a regular object already exists at the candidate's hash path, ingest
SHALL reuse that object without overwriting it, once its bytes pass the check
below. Concurrent ingests of matching bytes SHALL retain one complete object
at that path. This requirement concerns byte storage only. What a repeat
upload does to the library is set by the asset-library capability's
requirement "An upload of bytes already in the library finds the existing
Asset". Stored objects SHALL NOT be reclaimed by this slice.

#### Scenario: Matching bytes arrive again
- **WHEN** another accepted upload has the hash of an existing object whose bytes still match that hash
- **THEN** ingest reuses that object without changing it or creating another stored copy

#### Scenario: Concurrent matching bytes
- **WHEN** two requests concurrently publish the same candidate bytes
- **THEN** each successful commit refers to the same complete stored object
- **AND** neither request truncates or replaces the other's object
