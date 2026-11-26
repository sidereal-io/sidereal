# asset-page Specification

## Purpose

Let Nova open one image from the Library on its own page, at its own
address, and read the facts that Sidereal has recorded about it.

## Requirements

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

### Requirement: An upload in progress goes on while the asset page shows

An upload that is still sending when Nova opens an asset page SHALL keep
sending. The asset page SHALL show no upload status. When Nova returns to the
Library, it SHALL show that upload as it now stands: still in progress, or
its result. While it is in progress, the signed-in view SHALL send no
second upload request, on either screen.

#### Scenario: Nova opens an image while an upload is sending
- **GIVEN** Nova has chosen `m31.png` and its upload request is still pending
- **WHEN** Nova opens the asset page of `nebula.png` and then goes back to the Library
- **THEN** the asset page showed no upload status, and the upload request was not aborted
- **AND** the Library shows `Uploading m31.png…` and its Upload button is disabled
- **AND** choosing another file sends no second upload request

#### Scenario: The upload ends while the asset page shows
- **GIVEN** Nova opened the asset page of `nebula.png` while the upload of `m31.png` was pending
- **WHEN** the server answers that upload with 201, and Nova then goes back to the Library
- **THEN** the Library lists `m31.png` and shows no `Uploading` line

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
`image/jpeg` as `JPEG`, `image/tiff` as `TIFF`, and any other media type as
literal text, exactly as the server returns it.

#### Scenario: A JPEG's page
- **WHEN** Nova opens the asset page of an Asset whose media type is `image/jpeg`
- **THEN** the page's first fact line is `Format` with `JPEG`

#### Scenario: A TIFF's format
- **WHEN** Nova opens the asset page of an Asset whose media type is `image/tiff`
- **THEN** the page's first fact line is `Format` with `TIFF`

#### Scenario: A media type the page does not know
- **WHEN** Nova opens the asset page of an Asset whose media type is `image/x-test`
- **THEN** the page's first fact line is `Format` with `image/x-test`

#### Scenario: A media type holds markup
- **WHEN** an Asset's media type includes HTML markup
- **THEN** the `Format` line shows it as literal text and runs no markup

### Requirement: The asset page shows the image through its preview

The asset page SHALL show the image from the Asset's `preview_url`. When the
`preview_url` is `null`, it SHALL show the image from the `original_url`.
When the browser cannot show that image, "An image that cannot display
keeps its name on the page" applies. An answer whose `preview_url` is
neither `null` nor `/api/assets/<id>/preview` for the page's own Asset SHALL
count as not a valid Asset.

#### Scenario: A TIFF's page
- **WHEN** Nova opens the asset page of `m42.tif`, an 8-bit RGB TIFF of 800 by 600 pixels whose upload runs all succeeded
- **THEN** the page shows the image from `/api/assets/<id>/preview`, the name `m42.tif`, and these lines in this order: `Format` with `TIFF`, `File size` with its bytes, `Height` with `600 pixels`, and `Width` with `800 pixels`

#### Scenario: An image saved before this change
- **WHEN** Nova opens the asset page of `nebula.png`, saved before this change, whose `preview_url` is `null`
- **THEN** the page shows the image from `/api/assets/<id>/original`

#### Scenario: A TIFF whose preview run failed
- **WHEN** Nova opens the asset page of `cmyk.tif`, whose preview run failed, and the browser cannot show its original
- **THEN** the page shows its name, the line `Format` with `TIFF` and its other facts beside `This image could not be displayed.`, and no message about the failed run

#### Scenario: A preview address from elsewhere
- **WHEN** the server answers the asset page's request with a `preview_url` of `https://example.com/x.jpg`
- **THEN** the page shows `The image could not be loaded.` and a Retry button
