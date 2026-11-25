<!-- Plain language: Write questions and answers so a reader without the conversation can understand each decision and its source. Keep the confirmation marker exactly as written. -->

## Story

#525 image-format-and-dimensions — an uploaded image shows its format and pixel dimensions,
https://github.com/sidereal-io/sidereal/issues/525

## Decisions

| # | Question | Answer | Source |
|---|---|---|---|
| D1 | What does this story record? | The format and the pixel width and height of a PNG or JPEG, read from its bytes, not its name. EXIF, colour profiles and TIFF (#526) are out. | Evidence: #525 body |
| D2 | Where does the code that reads them live? | In the `packs/image` pack, which depends on `plugin-abi` only. | Evidence: #356 body, shared decision 1 |
| D3 | The public plugin contract has only one value type, a count. How is the format stored? | Add a choice value type to `plugin-abi`. Each facet schema declares its allowed values, so adding a format means a new schema version. | User |
| D4 | What does the format line read on the asset page? | The label `Format` with the value `PNG` or `JPEG`, and no unit. | User |
| D5 | How are the pixel dimensions recorded and shown? | As two count facets, `image.width` and `image.height`, with the unit `pixels`, shown as two lines. | User |
| D6 | Which Operator writes format and dimensions? | A new Operator, separate from the file-size Operator, so a damaged header fails only its run and the file size is still recorded. | User |
| D7 | What happens when a file has a valid signature but its header is missing or unreadable? | Its operation run is recorded as failed, with a message such as "the PNG header could not be read". The asset page shows the facts it has and no error. #527 shows processing failures. | User |
| D8 | What width and height does a JPEG with an EXIF rotation tag get? | The width and height stored in the file. Rotation is ignored, because EXIF is out of scope. | User |
| D9 | Do images uploaded before this change get format and dimensions? | No. There is no backfill in this story. They stay without them until #527 checks processing again at startup. | User |
| D10 | In what order do the facts show on the asset page? | Keep the server's order, sorted by facet name: File size, Format, Height, Width. A per-pack facet order is a decision for every pack, better made once there are more facets. | User |

## Changes from the story

- **Failure case narrowed.** The story says that a file whose bytes do not match an
  accepted format is recorded as an Operator failure. The upload Source already refuses
  files that are not PNG or JPEG, so the Operator never sees one. Its failure case is a
  file with a valid signature whose header is missing or unreadable (D7).
- **Added: a choice value type in `plugin-abi`** (D3). The public plugin contract had
  only a count, and the format needs a value from a declared set.
- No work was split out as a new issue.

Confirmed by user: yes
