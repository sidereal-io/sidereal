<!-- Plain language: Name the actor, condition and observable result. Explain necessary domain terms using the agreed glossary. Keep requirement and scenario headings and WHEN/THEN syntax exactly as required. -->

## ADDED Requirements

### Requirement: The asset page shows the image through its preview

The asset page SHALL show the image from the Asset's `preview_url`. When the
`preview_url` is `null`, it SHALL show the image from the `original_url`.
When the browser cannot show that image, "An image that cannot display
keeps its name on the page" applies. An answer whose `preview_url` is
neither `null` nor `/api/assets/<id>/preview` for the page's own Asset SHALL
count as not a valid Asset.

#### Scenario: A TIFF's page
- **WHEN** Nova opens the asset page of `m42.tif`, an 8-bit RGB TIFF of 800 by 600 pixels whose upload runs all succeeded
- **THEN** the page shows the image from `/api/assets/<id>/preview`, the name `m42.tif`, and these lines in this order: `File size` with its bytes, `Format` with `TIFF`, `Height` with `600 pixels`, and `Width` with `800 pixels`

#### Scenario: An image saved before this change
- **WHEN** Nova opens the asset page of `nebula.png`, saved before this change, whose `preview_url` is `null`
- **THEN** the page shows the image from `/api/assets/<id>/original`

#### Scenario: A TIFF whose preview run failed
- **WHEN** Nova opens the asset page of `cmyk.tif`, whose preview run failed, and the browser cannot show its original
- **THEN** the page shows its name and facts beside `This image could not be displayed.`, and no message about the failed run

#### Scenario: A preview address from elsewhere
- **WHEN** the server answers the asset page's request with a `preview_url` of `https://example.com/x.jpg`
- **THEN** the page shows `The image could not be loaded.` and a Retry button
