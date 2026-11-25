<!-- Plain language: Name the actor, condition and observable result. Explain necessary domain terms using the agreed glossary. Keep requirement and scenario headings and WHEN/THEN syntax exactly as required. -->

## MODIFIED Requirements

### Requirement: The asset page shows the image, its name and its facts

The asset page SHALL show the image whole, its name as literal text, and one
line for each facet the server returns, in the order the server returns
them. Each line SHALL show the facet's label, then its value, then its unit.
A count's value SHALL show with its digits grouped in threes by commas. A
choice's value SHALL show as literal text, exactly as the server returns it.
A line whose unit is empty SHALL show no unit. The value and unit SHALL be in
the mono face. The page SHALL show no provenance, and SHALL show no error
for a processing run that failed.

#### Scenario: A new upload's page
- **WHEN** Nova opens the asset page of `nebula.png`, uploaded after this change, a PNG of 1,234,567 bytes and 4,096 by 2,160 pixels
- **THEN** the page shows the image, the name `nebula.png`, and these lines in this order: `File size` with `1,234,567 bytes`, `Format` with `PNG`, `Height` with `2,160 pixels`, and `Width` with `4,096 pixels`
- **AND** each value and unit is in the mono face, and the `Format` line shows no unit

#### Scenario: The header run failed
- **WHEN** Nova opens the asset page of an image uploaded after this change whose file-size run succeeded with 1,000 bytes and whose header run failed
- **THEN** the page shows its image, its name and only the line `File size` with `1,000 bytes`
- **AND** the page shows no error and no `No facts are recorded for this image.` line

#### Scenario: An image with only its file size
- **WHEN** Nova opens the asset page of an image uploaded after Sidereal began recording file sizes but before this change, whose `image.file-size` is 1234567
- **THEN** the page shows its image, its name and only the line `File size` with `1,234,567 bytes`

#### Scenario: A name or label holds markup
- **WHEN** an Asset's name or a facet's label includes HTML markup
- **THEN** the page shows it as literal text and runs no markup

#### Scenario: A choice holds markup
- **WHEN** a choice facet's value includes HTML markup
- **THEN** the page shows it as literal text and runs no markup

### Requirement: The asset page says when no facts are recorded

When the server returns no facets for the Asset, the asset page SHALL show
the image and its name, and the line `No facts are recorded for this image.`
in place of the facts.

#### Scenario: An image uploaded before this change
- **WHEN** Nova opens the asset page of an image uploaded before Sidereal recorded any facts, which has no facet values
- **THEN** the page shows its image, its name and `No facts are recorded for this image.`

#### Scenario: The file-size run failed
- **WHEN** Nova opens the asset page of an image whose file-size run failed and whose header run failed too
- **THEN** the page shows its image, its name and `No facts are recorded for this image.`
