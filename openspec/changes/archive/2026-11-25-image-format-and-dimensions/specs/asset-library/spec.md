<!-- Plain language: Name the actor, condition and observable result. Explain necessary domain terms using the agreed glossary. Keep requirement and scenario headings and WHEN/THEN syntax exactly as required. -->

## MODIFIED Requirements

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
