<!-- Plain language: Name the actor, condition and observable result. Explain necessary domain terms using the agreed glossary. Keep requirement and scenario headings and WHEN/THEN syntax exactly as required. -->

## Purpose

Let Nova open one image from the Library on its own page, at its own
address, and read the facts that Sidereal has recorded about it.

## ADDED Requirements

### Requirement: An image in the Library opens its page

Each image in the Library SHALL be a link to its asset page, at the address
`/assets/<id>`, where `<id>` is the Asset's id. Following the link SHALL show
the asset page without reloading the document. The browser's Back action on
the asset page SHALL show the Library again.

#### Scenario: Nova opens an image
- **WHEN** Nova, signed in, clicks the image `nebula.png` in the Library
- **THEN** the address becomes `/assets/<id>` for that Asset and the asset page for `nebula.png` shows
- **AND** the document was not reloaded

#### Scenario: Back returns to the Library
- **WHEN** Nova uses the browser's Back action on an asset page they opened from the Library
- **THEN** the address is `/` and the Library shows

### Requirement: The asset page shows the image, its name and its facts

The asset page SHALL show the image whole, its name as literal text, and one
line for each facet the server returns. Each line SHALL show the facet's
label, then its value with digits grouped in threes by commas, then its unit.
The value and unit SHALL be in the mono face. The page SHALL show no
provenance.

#### Scenario: A new upload's page
- **WHEN** Nova opens the asset page of `nebula.png`, uploaded after this change, whose `image.file-size` is 1234567
- **THEN** the page shows the image, the name `nebula.png`, and the line `File size` with `1,234,567 bytes` in the mono face

#### Scenario: A name or label holds markup
- **WHEN** an Asset's name or a facet's label includes HTML markup
- **THEN** the page shows it as literal text and runs no markup

### Requirement: The asset page says when no facts are recorded

When the server returns no facets for the Asset, the asset page SHALL show
the image and its name, and the line `No facts are recorded for this image.`
in place of the facts.

#### Scenario: An image uploaded before this change
- **WHEN** Nova opens the asset page of an image uploaded before this change
- **THEN** the page shows its image, its name and `No facts are recorded for this image.`

#### Scenario: The file-size run failed
- **WHEN** Nova opens the asset page of an image whose file-size run failed
- **THEN** the page shows its image, its name and `No facts are recorded for this image.`

### Requirement: The asset page leads back to the Library

The asset page SHALL have a `Library` link that shows the Library at `/`
without reloading the document.

#### Scenario: Nova goes back by the link
- **WHEN** Nova follows the `Library` link on an asset page
- **THEN** the address is `/` and the Library shows, without a document reload

### Requirement: The asset page's address survives a reload

Opening `/assets/<id>` directly, or reloading it, SHALL show that Asset's
page to a signed-in administrator. A visitor with no valid session SHALL see
the access forms and no data of that Asset. After that visitor signs in, the
asset page for the address SHALL show.

#### Scenario: Nova reloads the page
- **WHEN** Nova, signed in, reloads `/assets/<id>` for `nebula.png`
- **THEN** the asset page for `nebula.png` shows again, with its facts

#### Scenario: A signed-out visitor opens the address
- **WHEN** a visitor with no valid session opens `/assets/<id>`
- **THEN** the page shows the sign-in form and no name, image or facet of that Asset
- **AND** after the visitor signs in, the asset page for that Asset shows

### Requirement: The asset page's states are distinct

While it waits for the server, the asset page SHALL show
`Loading the image…`. When the server answers 404, it SHALL show
`This image is not in the library.` and the `Library` link. When the request
fails, or the answer is not a valid Asset with facets, it SHALL show
`The image could not be loaded.` and a Retry button that asks again. A
failed load SHALL NOT show the no-facts line.

#### Scenario: An Asset that does not exist
- **WHEN** Nova opens `/assets/<id>` with an id that no Asset has
- **THEN** the page shows `This image is not in the library.` and the `Library` link

#### Scenario: The server cannot answer
- **WHEN** the request for the Asset returns 503, fails on the network, or returns a body that is not a valid Asset with facets
- **THEN** the page shows `The image could not be loaded.` and a Retry button, not `No facts are recorded for this image.`
- **AND** pressing Retry asks the server again

### Requirement: An image that cannot display keeps its name on the page

When the browser cannot load or decode the image, the asset page SHALL keep
the name and the facts visible and show `This image could not be displayed.`
in the image's place.

#### Scenario: The image cannot render
- **WHEN** the asset page's image fails to load in the browser
- **THEN** its name and facts remain visible beside `This image could not be displayed.`

### Requirement: Ending the session hides the asset page

Signing out or losing the session on the asset page SHALL hide the Asset's
data. An answer that arrives later SHALL NOT show it again.

#### Scenario: The session ends while the page loads
- **WHEN** the session ends while the asset page's request is pending
- **THEN** the page shows the sign-in form, and the later answer shows no data of the Asset

### Requirement: The asset page follows DESIGN.md

The asset page SHALL follow the asset page section of DESIGN.md and use only
its tokens. Its image SHALL have square corners and SHALL NOT be cropped.

#### Scenario: The page uses tokens
- **WHEN** the web unit tests read the asset page's stylesheet
- **THEN** every color, size, spacing and font value it uses is a token that DESIGN.md defines
