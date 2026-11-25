<!-- Plain language: Write questions and answers so a reader without the conversation can understand each decision and its source. Keep the confirmation marker exactly as written. -->

## Story

#525 image-format-and-dimensions — an uploaded image shows its format and pixel dimensions,
https://github.com/sidereal-io/sidereal/issues/525

## Decisions

| # | Question | Answer | Source |
|---|---|---|---|
| D1 | What does this story record? | The format and the pixel width and height of a PNG or JPEG, read from its bytes, not its name. EXIF, colour profiles and TIFF (#526) are out. | Evidence: #525 body |
| D2 | Where does the code that reads them live? | In the `packs/image` pack, which depends on `plugin-abi` only. | Evidence: #356 body, shared decision 1 |
| D3 | The public plugin contract has only one value type, a count. How is the format recorded? | It is not a facet. The asset page shows the format from the version's `media_type`, which the upload Source already sets from the file's bytes. A closed choice owned by `packs/image` could not cross packs: under ADR-008 one pack owns a facet's schema, so `packs/astro` could never add FITS or XISF. `plugin-abi` gets no choice type in this story; one comes with its first real user, such as `astro.frame_type`. This replaces the earlier answer, a choice value type in `plugin-abi`, after the pull request review. | User |
| D4 | What does the format line read on the asset page? | The label `Format` with the format's common name and no unit: `image/png` shows as `PNG`, `image/jpeg` as `JPEG`. A media type the web does not know shows as it is. | User |
| D5 | How are the pixel dimensions recorded and shown? | As two count facets, `image.width` and `image.height`, with the unit `pixels`, shown as two lines. | User |
| D6 | Which Operator writes the dimensions? | A new Operator, separate from the file-size Operator, so a damaged header fails only its run and the file size is still recorded. | User |
| D7 | What happens when a file has a valid signature but its header is missing or unreadable? | Its operation run is recorded as failed, with a message such as "the PNG header could not be read". The asset page shows the facts it has and no error. #527 shows processing failures. | User |
| D8 | What width and height does a JPEG with an EXIF rotation tag get? | The width and height stored in the file. Rotation is ignored, because EXIF is out of scope. | User |
| D9 | Do images uploaded before this change get dimensions? | No. There is no backfill in this story. They stay without them until #527 checks processing again at startup. They do show their `Format`, because every version already has a media type (D3). | User |
| D10 | In what order do the facts show on the asset page? | `Format` first, for every image, then the server's facets in its order, sorted by facet name: File size, Height, Width. A per-pack facet order is a decision for every pack, better made once there are more facets. | User |
| D11 | Where does the server give the web the media type? | `GET /api/assets/{id}` adds `media_type`, the current version's media type such as `image/png`, to its `asset` object only. The library list and the upload response keep their shape. | User |
| D12 | What happens to the line `No facts are recorded for this image.`? | It goes. Every version has a media type, so every asset page has a `Format` line and the facts are never empty. An image whose runs all failed shows only its `Format` line. | User |

## Changes from the story

- **Failure case narrowed.** The story says that a file whose bytes do not match an
  accepted format is recorded as an Operator failure. The upload Source already refuses
  files that are not PNG or JPEG, so the Operator never sees one. Its failure case is a
  file with a valid signature whose header is missing or unreadable (D7).
- **Format is shown, not recorded.** The story says the format is recorded
  with the dimensions. The asset page shows it from the version's media type
  instead, and only the width and height are facets (D3, D11).
- No work was split out as a new issue.

Confirmed by user: yes
