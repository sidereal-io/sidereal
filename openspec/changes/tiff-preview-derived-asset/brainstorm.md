<!-- Plain language: Write questions and answers so a reader without the conversation can understand each decision and its source. Keep the confirmation marker exactly as written. -->

## Story

#526 tiff-preview-derived-asset — a TIFF upload shows in the browser through a preview made as a derived asset. https://github.com/sidereal-io/sidereal/issues/526

## Decisions

| # | Question | Answer | Source |
|---|---|---|---|
| D1 | Does every upload get a preview, or only a TIFF? | Every format gets a preview asset, as the packet says. A PNG or JPEG is stored twice: once as the original, once as the preview. | User |
| D2 | Does #526 stay one story, or split into a PNG/JPEG preview story and a TIFF story? | Keep it as one story. | User |
| D3 | What file format is the preview? | An 8-bit JPEG at high quality, about 90. Transparent areas turn black. | User |
| D4 | Which TIFFs are accepted, and which get a preview? | Upload accepts any file with a TIFF signature: classic TIFF or BigTIFF, either byte order. The preview uses the first image only, and decodes grayscale or RGB in 8-bit or 16-bit whole numbers or 32-bit decimals, uncompressed or LZW, Deflate or PackBits. Any other TIFF still becomes an asset with its facets; its preview run is recorded as failed with a reason, and the page says "This image could not be displayed." | User |
| D5 | How does a 16-bit or 32-bit TIFF become an 8-bit preview? | Scale straight across with no brightening: 16-bit values map onto 0–255; decimal values from 0 to 1 map onto 0–255, with values outside clipped. Display-only auto-stretch belongs with FITS (#357). | User |
| D6 | What memory limit applies to making a preview? | 512 MiB per preview decode, including decoded pixels and decoder scratch space. If decoding would need more, the preview run fails with a clear reason. | User |
| D7 | Does the upload's answer wait until the preview is made? | Yes, as it already waits for the file-size and header runs. | User |
| D8 | What shows for an asset with no preview (uploaded before this change, or its preview run failed)? | The original: older PNGs and JPEGs still display; a TIFF shows "This image could not be displayed". No backfill. | User |
| D9 | Do the upload Operators (file size, header, preview) also run on a preview? | No. A preview gets no facets and never gets a preview of its own. | User |
| D10 | What happens when someone uploads bytes that exactly match an existing preview? | The upload makes a new original asset, because the repeat-upload check looks only at originals. | User |
| D11 | Glossary: what is a Preview? | A derived asset holding a browser-ready image made from one asset version at full resolution. The library and asset page show it in place of the original. Written to `openspec/glossary.md`. | User |
| D12 | Glossary: what is an Original? | An asset a Source brought into the library, as opposed to a derived asset. The library lists originals only. Written to `openspec/glossary.md`. | User |
| D13 | How does the image pack hold the code for each format? | One module per format: `formats/png.rs`, `formats/jpeg.rs` and `formats/tiff.rs`, behind an internal `ImageFormat` trait. The trait gives a format's name and media type, how many leading bytes it needs and whether they match, how to read its dimensions, and how to render a preview. `CandidateReader` moves to a format-agnostic `io.rs`. One `FORMATS` table drives recognition, registration and the refusal message, so adding a format touches one module and one table entry. Each format keeps its own read limits and memory tests beside its code. The Source reads as many leading bytes as the formats need, not a fixed 8. | User |
| D14 | Who owns the list of formats that upload takes? | The image pack. The server passes the Source's own refusal reason to the client, in place of its fixed "only PNG and JPEG" text. The web names no formats: it shows the reason the server gives. | User |
| D15 | How does an Operator learn a version's format, and which versions does it run on? | The Asset context gives an Operator the version's media type, so the header Operator no longer reads the signature the Source already read. Each Operator declares the media types it accepts, and core runs an upload's Operators only on a version whose media type they accept. This is a first, small step towards the `accepts` Selector and covers media types only. It also keeps Operators off a preview (D9): no upload Operator accepts a preview's media type in the preview's role. | User |
| D16 | How does a TIFF show its format, now that #547 records no format facet? | From the version's media type: `image/tiff` shows as `Format TIFF`, through the asset page's map from media type to name that #547 added. This change adds no `image.format` facet and no choice type. | User |
| D17 | How does Sidereal enforce the decode budget? | Run the built-in preview decoder and encoder in a short-lived, memory-capped child process. The upload still waits for the run and the Operator still reads and writes through its Asset context. | User |
| D18 | Does a JPEG whose scan data stops short, but which still ends in EOI, fail its preview? | No. It gets a best-effort preview, filled in by the decoder as browsers do. A JPEG with no EOI still fails. No JPEG decoder that fits the plan refuses short scan data, upstream zune-jpeg has no strict option for it, and Sidereal will not patch the dependency. | User |

## Changes from the story

- **Added:** two glossary entries, Preview and Original, under "Assets and processing" in `openspec/glossary.md`, committed on this branch. The packet expected the glossary terms to land with #521; these two are new.
- **Added:** limits the packet did not state: which TIFF variants get a preview (D4), the 512 MiB decode limit (D6), and a failed preview run with a reason when either is not met.
- **Cut:** backfilling previews for assets uploaded before this change (D8), and any automatic brightening of raw data (D5, left to FITS in #357).
- No split: the story stays one change (D2).
- **Reworked after #547 changed:** #547 dropped its `image.format` facet and its choice type, and shows the format from the media type. This change no longer adds TIFF to a choice; it adds `TIFF` to the asset page's names (D16).
- **Added after the plan:** one module per format in the image pack (D13), the pack's own refusal reason on the upload screen (D14), and Operators that declare the media types they accept (D15).
- **Changed during plan review:** a JPEG that still ends in EOI gets a best-effort preview when its scan data stops short, instead of failing (D18).

Confirmed by user: yes
