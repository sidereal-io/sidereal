<!-- Plain language: Write questions and answers so a reader without the conversation can understand each decision and its source. Keep the confirmation marker exactly as written. -->

## Story

#522 same-bytes-find-existing-asset — uploading an image already in the library opens the existing one instead of making a copy: https://github.com/sidereal-io/sidereal/issues/522

## Decisions

| # | Question | Answer | Source |
|---|---|---|---|
| D1 | Which existing Asset counts as "already in the library" for an upload? | Any Asset with a version whose BLAKE3 hash matches, whichever Source brought it in. #526 decides whether a derived asset's (preview's) bytes count. | User |
| D2 | If two Assets already share the bytes (only on development databases built from #521), which one does a repeat upload find? | The oldest: earliest `ingested_at` version, then the lowest id. Both Assets stay as they are. | User |
| D3 | When a repeat upload has another filename, what happens to the existing Asset's name? | It keeps its name. The new filename is discarded and no alias is recorded. Renaming belongs to #359. | User |
| D4 | What does `POST /api/assets` answer for a repeat upload? | 409 Conflict. A first upload still answers 201. | User |
| D5 | What does "the library screen opens the existing image" mean now, with no asset page or viewer yet? | Nothing in this story. The viewer (#528) is not built, so this story builds no "open" step. | User |
| D6 | Should the lost-answer message change now that a repeat upload is safe? | Yes: say it is safe to upload the file again, because Sidereal finds it if it was saved. Still never retry automatically. | User |
| D7 | Two uploads of the same bytes at the same moment: what happens? | One Asset is made. The other upload is a repeat upload and answers as D4 says. | Evidence: #522 packet scope, with D4 |
| D8 | Is anything else about duplicates in scope? | No: finding near-duplicates and merging Assets made before this story stay out. | Evidence: #522 packet scope |
| D9 | What does the body of the 409 answer for a repeat upload contain? | An `asset` object for the existing Asset, with the same fields as the 201 answer and the library list: `id`, `kind`, `name`, `current_version_id`, `original_url`. | User |
| D10 | What does the library screen show when an upload answers 409? | A notice that is not a refusal: "<chosen name> is already in the library." When the existing Asset has another name, it adds " as <existing name>", for example "m31-final.png is already in the library as nebula.png." The list does not change, and Nova can choose another file at once. | User |

## Changes from the story

- Cut: "the library screen opens the existing image". There is no asset page or viewer to open yet; #528 brings the viewer (D5).
- Added: the lost-answer message now says a second upload is safe (D6).

Confirmed by user: yes
