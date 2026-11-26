# derived-assets Specification

## Purpose

Let an Operator make a new asset from the Asset version it reads, through the
public contract, while core keeps the asset's identity, hash, provenance and
lineage link. The first such asset is the preview.

## Requirements

### Requirement: An Operator declares that it makes previews when it registers

An Operator SHALL declare, when it registers, whether it makes previews. The
Registry SHALL keep that answer from registration, as it keeps the
Operator's id, version and grants. An Operator that declares nothing SHALL
make no previews.

#### Scenario: An Operator that makes previews
- **WHEN** a test pack registers an Operator that declares it makes previews
- **THEN** the Registry records that this Operator makes previews

#### Scenario: An Operator that declares nothing
- **WHEN** a test pack registers an Operator that declares only facets
- **THEN** the Registry records that this Operator makes no previews

### Requirement: An Operator proposes a preview through its Asset context

During its run, an Operator SHALL be able to propose one preview of the
version it reads, through its Asset context. The proposal SHALL hold only a
kind and a media type. The Operator SHALL write the preview's bytes through
the same context. The context SHALL give it no path, id, hash or provenance
to set, and no way to name another version as the preview's source. A
Source run SHALL NOT be able to propose a preview.

#### Scenario: A test Operator proposes a preview
- **WHEN** a test Operator that declares it makes previews reads a stored PNG version, proposes a preview of kind `image` and media type `image/png`, and writes 10 bytes through its context
- **THEN** its run is `succeeded`
- **AND** a new derived Asset of kind `image` exists, whose one version holds those 10 bytes with the media type `image/png`

#### Scenario: A Source proposes a preview
- **WHEN** a test Source proposes a preview during its run
- **THEN** the context refuses the proposal, and the run fails with no Asset committed

### Requirement: Core checks a proposed preview before it stores it

Core SHALL record the run `failed`, with a reason, and commit no Asset, no
Asset version and no lineage link, when any of these holds:

- the Operator did not declare that it makes previews;
- the run proposed more than one preview;
- the run wrote bytes without proposing a preview, or proposed a preview
  and wrote no bytes;
- the preview's media type is not `image/jpeg` or `image/png`;
- the preview's bytes are more than 100 MiB (104,857,600 bytes).

#### Scenario: An Operator that did not declare previews
- **WHEN** a test Operator that declares no previews proposes a preview and writes its bytes
- **THEN** its run is `failed` with a reason that says the Operator does not make previews
- **AND** no new Asset exists

#### Scenario: A second preview in one run
- **WHEN** a test Operator that declares it makes previews proposes two previews in one run
- **THEN** its run is `failed` with a reason that says it proposed more than one preview
- **AND** no new Asset exists

#### Scenario: A preview with no bytes
- **WHEN** a test Operator that declares it makes previews proposes a preview and writes no bytes
- **THEN** its run is `failed` with a reason that says the preview is empty
- **AND** no new Asset exists

#### Scenario: A preview of a media type a browser may not show
- **WHEN** a test Operator that declares it makes previews proposes a preview of media type `image/tiff`
- **THEN** its run is `failed` with a reason that names `image/tiff`
- **AND** no new Asset exists

#### Scenario: A preview over 100 MiB
- **WHEN** a test Operator that declares it makes previews proposes a preview and writes 104,857,601 bytes
- **THEN** its run is `failed` with a reason that says the preview is larger than 100 MiB
- **AND** no new Asset exists, and no staging file is left

### Requirement: Core records a preview as a derived asset with lineage

When a run's preview passes core's checks, core SHALL:

- create a new Asset, marked derived, with the proposed kind and the name of
  the Asset whose version the run read;
- create its one version, with the hash and size core computed from the
  bytes written, the proposed media type, and the Operator's id and version
  as its provenance;
- record one lineage link from the new version to the version the run read,
  as its preview, naming the run.

Core SHALL mint every id.

#### Scenario: The recorded preview
- **WHEN** a test Operator that declares it makes previews runs over the version of `nebula.png` and writes a preview of media type `image/jpeg`
- **THEN** a new Asset named `nebula.png` exists, marked derived, with one version whose hash is the BLAKE3 hash of the bytes written and whose provenance names the test Operator's id and version
- **AND** one lineage link exists from that version to the version of `nebula.png`, as its preview, naming the run, which is `succeeded`

### Requirement: A preview commits with its run

Core SHALL publish the preview's bytes to managed storage first, as an
uploaded file's are. It SHALL then create the preview's Asset, version and
lineage link in the transaction that records the run `succeeded` and stores
its facet values. When that transaction does not commit, none of them SHALL
exist.

#### Scenario: The database refuses the commit
- **WHEN** a test Operator writes a valid preview and PostgreSQL refuses the transaction that would record it
- **THEN** no new Asset, Asset version or lineage link exists, and the run is not `succeeded`

### Requirement: A failed write invalidates the preview

A failed staging write SHALL invalidate the preview, even when the
Operator continues or reports success. Core SHALL record the staging
reason when the Operator reports success or reports that a preview write
stopped it. When the Operator fails for another reason, core SHALL record
the Operator's reason.

#### Scenario: An Operator ignores a storage write failure
- **WHEN** a test Operator writes part of a preview, ignores a storage error on its next write and reports success
- **THEN** its run is `failed` with the reason `the preview could not be stored`
- **AND** no preview is published and no new Asset, version or lineage link exists

#### Scenario: An Operator fails for its own reason after a write failure
- **WHEN** a test Operator's preview write fails, and the Operator then fails with the reason `stopped early`
- **THEN** its run is `failed` with the reason `stopped early`
- **AND** no preview is published

### Requirement: Cancellation before the preview transaction stops the run

Core SHALL recheck cancellation after publication and before it starts
recording the database outcome. Once recording starts, the database
transaction SHALL decide the outcome.

#### Scenario: The run is cancelled while preview publication is in progress
- **WHEN** the host cancels a preview run while core is publishing its bytes
- **THEN** the run is `failed` with a reason that says it was cancelled
- **AND** no new Asset, version or lineage link exists

#### Scenario: The run is cancelled after it proposed a preview
- **WHEN** the host cancels a run after its Operator proposed a preview and wrote its bytes
- **THEN** the run is `failed` with a reason that says it was cancelled
- **AND** no new Asset, Asset version or lineage link exists

### Requirement: A refused proposal stores nothing from the run

When core refuses any facet value or the preview that a run proposed, it
SHALL store neither the run's facet values nor its preview.

#### Scenario: A refused facet stops the preview too
- **WHEN** a test Operator that declares it makes previews proposes a valid preview and a value for a facet it was not granted
- **THEN** its run is `failed`, no new Asset exists, and no facet value from the run is stored

### Requirement: A version's preview is its newest

The preview of an Asset version SHALL be the derived Asset linked to it as
its preview by the run with the latest recorded completion time. Core SHALL
record that time when it marks the run succeeded. If times tie, the greater
run id in database order SHALL win. A version with no such link SHALL have
no preview.

#### Scenario: Two preview runs over one version
- **WHEN** core runs a preview Operator twice over the same version, and both runs succeed
- **THEN** two derived Assets exist, and the version's preview is the one the second run made

#### Scenario: A version with no preview
- **WHEN** no preview run has succeeded over a version
- **THEN** that version has no preview

### Requirement: Core keeps which Assets are originals

Core SHALL record, for each Asset, whether it is an original, brought in by
a Source, or a derived Asset, made by an Operator run. Every Asset that
existed before this change SHALL be an original.

#### Scenario: An uploaded image
- **WHEN** Nova uploads `nebula.png` and the server answers 201
- **THEN** the Asset of `nebula.png` is an original

#### Scenario: An Asset saved before this change
- **WHEN** the server starts with this change over a database that already holds the Asset of `m31.png`
- **THEN** the Asset of `m31.png` is an original, with the same id, name, kind and current version as before
