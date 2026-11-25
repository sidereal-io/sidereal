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
| D6 | What memory limit applies to making a preview? | 512 MiB. If decoding would need more, the preview run fails with a clear reason. | User |
| D7 | Does the upload's answer wait until the preview is made? | Yes, as it already waits for the file-size and header runs. | User |
| D8 | What shows for an asset with no preview (uploaded before this change, or its preview run failed)? | The original: older PNGs and JPEGs still display; a TIFF shows "This image could not be displayed". No backfill. | User |
| D9 | Do the upload Operators (file size, header, preview) also run on a preview? | No. A preview gets no facets and never gets a preview of its own. | User |
| D10 | What happens when someone uploads bytes that exactly match an existing preview? | The upload makes a new original asset, because the repeat-upload check looks only at originals. | User |
| D11 | Glossary: what is a Preview? | A derived asset holding a browser-ready image made from one asset version at full resolution. The library and asset page show it in place of the original. Written to `openspec/glossary.md`. | User |
| D12 | Glossary: what is an Original? | An asset a Source brought into the library, as opposed to a derived asset. The library lists originals only. Written to `openspec/glossary.md`. | User |

## Changes from the story

- **Added:** two glossary entries, Preview and Original, under "Assets and processing" in `openspec/glossary.md`, committed on this branch. The packet expected the glossary terms to land with #521; these two are new.
- **Added:** limits the packet did not state: which TIFF variants get a preview (D4), the 512 MiB decode limit (D6), and a failed preview run with a reason when either is not met.
- **Cut:** backfilling previews for assets uploaded before this change (D8), and any automatic brightening of raw data (D5, left to FITS in #357).
- No split: the story stays one change (D2).

Confirmed by user: yes
