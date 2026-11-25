<!-- Plain language: Name the actor, condition and observable result. Explain necessary domain terms using the agreed glossary. Keep requirement and scenario headings and WHEN/THEN syntax exactly as required. -->

## ADDED Requirements

### Requirement: The asset page shows no processing failure

When a processing run for the Asset failed, the asset page SHALL show the
`Format` line and the facts the server returns, and SHALL show no message
about that run. When the image's bytes cannot be displayed, the image area
SHALL still show `This image could not be displayed.`, as for any image.

#### Scenario: The header run failed
- **WHEN** Nova opens the asset page of a PNG uploaded after this change whose file-size run succeeded with 1,000 bytes and whose header run failed
- **THEN** the page shows its name, the line `Format` with `PNG`, and then only the line `File size` with `1,000 bytes`
- **AND** the page shows no message about the failed run

#### Scenario: Every run failed
- **WHEN** Nova opens the asset page of a PNG whose file-size run and header run both failed, so the server returns no facets
- **THEN** the page shows its image, its name and only the line `Format` with `PNG`

### Requirement: The asset page shows the image's format

The first fact line of every asset page SHALL be `Format`, from the Asset's
media type, with no unit. The media type `image/png` SHALL show as `PNG`,
`image/jpeg` as `JPEG`, and any other media type as literal text, exactly as
the server returns it.

#### Scenario: A JPEG's page
- **WHEN** Nova opens the asset page of an Asset whose media type is `image/jpeg`
- **THEN** the page's first fact line is `Format` with `JPEG`

#### Scenario: A media type the page does not know
- **WHEN** Nova opens the asset page of an Asset whose media type is `image/x-test`
- **THEN** the page's first fact line is `Format` with `image/x-test`

#### Scenario: A media type holds markup
- **WHEN** an Asset's media type includes HTML markup
- **THEN** the `Format` line shows it as literal text and runs no markup

## MODIFIED Requirements

### Requirement: The asset page shows the image, its name and its facts

The asset page SHALL show the image whole, its name as literal text, and
then its facts: the `Format` line, then one line for each facet the server
returns, in the server's order. Each facet line SHALL show the facet's
label, then its value with digits grouped in threes by commas, then its
unit. A line with an empty unit SHALL show no unit. Every value and unit
SHALL be in the mono face. The page SHALL show no provenance.

#### Scenario: A new upload's page
- **WHEN** Nova opens the asset page of `nebula.png`, uploaded after this change, a PNG of 1,234,567 bytes and 4,096 by 2,160 pixels
- **THEN** the page shows the image, the name `nebula.png`, and these lines in this order: `Format` with `PNG`, `File size` with `1,234,567 bytes`, `Height` with `2,160 pixels`, and `Width` with `4,096 pixels`
- **AND** each value and unit is in the mono face, and the `Format` line shows no unit

#### Scenario: An image with only its file size
- **WHEN** Nova opens the asset page of a PNG uploaded after Sidereal began recording file sizes but before this change, whose `image.file-size` is 1234567
- **THEN** the page shows its image, its name, the line `Format` with `PNG` and then only the line `File size` with `1,234,567 bytes`

#### Scenario: An image uploaded before Sidereal recorded any facts
- **WHEN** Nova opens the asset page of a JPEG that has no facet values
- **THEN** the page shows its image, its name and only the line `Format` with `JPEG`

#### Scenario: A name or label holds markup
- **WHEN** an Asset's name or a facet's label includes HTML markup
- **THEN** the page shows it as literal text and runs no markup

### Requirement: The asset page's states are distinct

While it waits for the server, the asset page SHALL show
`Loading the image…`. When the server answers 404, it SHALL show
`This image is not in the library.` and the `Library` link. When the request
fails, or the answer is not a valid Asset with a media type and facets, it
SHALL show `The image could not be loaded.` and a Retry button that asks
again. A valid media type is text that is not empty. A failed load SHALL NOT
show a `Format` line or any other fact.

#### Scenario: Nova moves from one asset page to another
- **GIVEN** the asset page of `nebula.png` has loaded, and the request for `m31.png`'s Asset is still pending
- **WHEN** the address changes from `nebula.png`'s asset page to `m31.png`'s, through the browser's history
- **THEN** the page shows `Loading the image…` and no name, image or fact of `nebula.png`

#### Scenario: An Asset that does not exist
- **WHEN** Nova opens `/assets/<id>` with an id that no Asset has
- **THEN** the page shows `This image is not in the library.` and the `Library` link

#### Scenario: The server cannot answer
- **WHEN** the request for the Asset returns 503, fails on the network, or returns a body that is not a valid Asset with facets
- **THEN** the page shows `The image could not be loaded.` and a Retry button, and no `Format` line
- **AND** pressing Retry asks the server again

#### Scenario: An answer without a media type
- **WHEN** the request for the Asset returns 200 with an Asset whose `media_type` is missing, empty, or not text
- **THEN** the page shows `The image could not be loaded.` and a Retry button, and no `Format` line

## REMOVED Requirements

### Requirement: The asset page says when no facts are recorded
**Reason**: Every Asset version has a media type, so the asset page always shows a `Format` line, and its facts are never empty.
**Migration**: An Asset with no facet values shows only its `Format` line. "The asset page shows the image, its name and its facts" covers it in the scenario "An image uploaded before Sidereal recorded any facts", and "The asset page shows no processing failure" covers an Asset whose runs all failed.
