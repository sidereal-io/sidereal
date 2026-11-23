<!-- Plain language: Name the actor, condition and observable result. Explain necessary domain terms using the agreed glossary. Keep requirement and scenario headings and WHEN/THEN syntax exactly as required. -->

## Purpose

Let a plugin record checked facts about an Asset version through the public
Operator contract, with core as the only writer of facet values and the
keeper of their provenance and of each operation run.

## ADDED Requirements

### Requirement: An Operator runs through the public contract

Core SHALL run an Operator that a pack registered through the public
Registry, over one Asset version. The Operator SHALL reach the version only
through a run-scoped Asset context. Through it, the Operator SHALL be able to
read the version's bytes, propose facet values, write log lines and check
whether the run was cancelled. The context SHALL give the Operator no
database handle, no path in the object store, and no way to set an id, a
hash or provenance.

#### Scenario: A test Operator registered through the public contract
- **WHEN** a test pack registers an Operator that reads the version's first byte through the context and proposes it as the value of a facet it declares, and core runs it over a stored version
- **THEN** core stores that value for that version
- **AND** the stored provenance names the test Operator's id and version

#### Scenario: Log lines go through the host
- **WHEN** an Operator writes the line `looked at the bytes` through its context
- **THEN** the host receives that line, and the Operator wrote it nowhere else

### Requirement: Each facet schema has one owning pack

A pack SHALL declare each facet schema it owns when it registers. A facet
schema SHALL give the facet's name, label, value type, unit, version, and
whether it describes an Asset or an Asset version. A facet's name SHALL
start with its owning pack's id and a dot, as `image.file-size` starts with
`image.`. The Registry SHALL refuse a declaration whose name another
declaration already uses, or whose name does not start with its pack's id
and a dot.

#### Scenario: Two packs declare the same facet
- **WHEN** one test pack has declared the facet `test.size` and a second pack declares `test.size`
- **THEN** the Registry refuses the second declaration with an error that names `test.size`

#### Scenario: A pack declares a facet outside its namespace
- **WHEN** the pack `test` declares a facet named `other.size`
- **THEN** the Registry refuses the declaration with an error that names `other.size`

### Requirement: An Operator writes only the facets it declares

An Operator SHALL list the facets it writes when its pack registers it. That
list is the Operator's grants. Until plugin installation adds an approval
step, a pack compiled into the server SHALL have every grant its Operators
list. Core SHALL refuse a proposed value for a facet that is not in the
proposing Operator's list.

#### Scenario: An Operator proposes a facet it did not declare
- **WHEN** an Operator that lists only `test.size` proposes a value for `test.width`, which another schema declares
- **THEN** the run is recorded as failed with a reason that names `test.width`
- **AND** core stores no facet value from that run

### Requirement: Core checks every proposal before it stores any

Core SHALL refuse a proposed value whose facet has no registered schema, or
whose value does not fit the schema's value type. The only value type is a
whole number of zero or more. When core refuses any proposal from a run, it
SHALL record the run as failed and SHALL store none of that run's proposals.

#### Scenario: An unknown facet
- **WHEN** an Operator that lists `test.missing` proposes a value for it, and no pack declares a schema named `test.missing`
- **THEN** the run is recorded as failed with a reason that names `test.missing`
- **AND** core stores no facet value from that run

#### Scenario: A value that does not fit its type
- **WHEN** an Operator proposes the value -1 for a facet whose value type is a whole number of zero or more
- **THEN** the run is recorded as failed with a reason that names the facet
- **AND** core stores no facet value from that run

#### Scenario: One refused proposal among valid ones
- **WHEN** an Operator proposes a valid value for `test.size` and then a value for a facet it did not declare
- **THEN** the run is recorded as failed
- **AND** core stores neither value

### Requirement: Every run is recorded as an operation run

Core SHALL record one operation run for each time it runs an Operator. The
record SHALL hold its own opaque id, the Operator's id and version, the Asset
version it read, when it started and ended, and how it ended: `succeeded` or
`failed`. A failed run SHALL record its reason. A run SHALL be `failed` when
the Operator reports a failure, when it stops abnormally, when the host
cancels it, or when core refuses a proposal. A run that ends with no
proposals and no failure SHALL be `succeeded`.

#### Scenario: A successful run
- **WHEN** core runs an Operator that proposes one valid value
- **THEN** one operation run exists with outcome `succeeded`, the Operator's id and version, the version it read, and an end time no earlier than its start time

#### Scenario: The Operator reports a failure
- **WHEN** an Operator returns the failure `could not read the header`
- **THEN** its operation run is `failed` with the reason `could not read the header`
- **AND** core stores no facet value from that run

#### Scenario: The Operator stops abnormally
- **WHEN** an Operator panics during its run
- **THEN** its operation run is `failed` with a reason that says the Operator stopped
- **AND** the server goes on answering requests

#### Scenario: The host cancels the run
- **WHEN** the host cancels a run after the Operator proposed a valid value
- **THEN** the operation run is `failed` with a reason that says it was cancelled
- **AND** core stores no facet value from that run

### Requirement: Stored facet values carry their provenance

Core SHALL be the only writer of facet values. With each value it stores,
core SHALL record the producing plugin's id and version, the operation run
that produced it, the facet schema's version, and when core stored it. Core
SHALL take the producer from the registered Operator that ran, never from
anything the Operator proposed. Core SHALL store a value for an Asset version
facet against the version the run read.

#### Scenario: Provenance of a stored value
- **WHEN** a run of a test Operator at version `1.2.3` succeeds with one value for a version facet at schema version 1
- **THEN** the stored value names the test Operator, version `1.2.3`, that run's id and schema version 1, and has the time it was stored
- **AND** it belongs to the Asset version the run read

### Requirement: The image pack records an image's file size

The image pack SHALL declare the facet schema `image.file-size`, with the
label `File size`, a whole number of zero or more as its value type, the unit
`bytes`, schema version 1, and the Asset version as what it describes. The
image pack SHALL register an Operator that lists `image.file-size` and
proposes, as its value, the number of bytes in the version it reads.

#### Scenario: A PNG of known size
- **WHEN** core runs the file-size Operator over a stored PNG version of 1,234,567 bytes
- **THEN** the version's `image.file-size` value is 1234567
- **AND** its provenance names the file-size Operator and the image pack's version
