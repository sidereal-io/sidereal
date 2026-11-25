## ADDED Requirements

### Requirement: An Operator's values stay on its own input

The Asset context SHALL give an Operator no way to name the Asset or Asset
version that a proposed value belongs to. Core SHALL decide that from the
run's input alone, and SHALL store no value from a run against any Asset or
Asset version other than the version the run read and the Asset that owns
it. This includes another version of the same Asset.

#### Scenario: Other Assets and versions exist
- **WHEN** the library holds Asset A with versions A1 and A2, and Asset B with version B1, and a run over A1 succeeds with one value for an Asset version facet and one value for an Asset facet
- **THEN** the Asset version value belongs to A1 and the Asset value belongs to A
- **AND** no value from that run belongs to A2, B or B1

### Requirement: An Operator run makes no Asset

Core SHALL record an Operator run that proposes an Asset as failed. Core
SHALL add no Asset and no Asset version for that run, and SHALL store no
facet value from it. When the Operator reports success after proposing an
Asset, the reason SHALL say that an Operator run proposed an Asset.

#### Scenario: An Operator proposes an Asset and reports success
- **WHEN** an Operator proposes a valid value for `test.size`, then proposes an Asset of kind `image`, ignores the refusal and reports success
- **THEN** its operation run is `failed` with a reason that says an operator run proposed an asset
- **AND** the library holds the same Assets and Asset versions as before the run
- **AND** core stores no facet value from that run

#### Scenario: An Operator proposes an Asset and stops at the refusal
- **WHEN** an Operator proposes a valid value for `test.size`, then proposes an Asset of kind `image`, and returns the failure `could not make a preview` when the context refuses it
- **THEN** its operation run is `failed` with the reason `could not make a preview`
- **AND** the library holds the same Assets and Asset versions as before the run
- **AND** core stores no facet value from that run
