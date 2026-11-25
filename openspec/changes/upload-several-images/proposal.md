## Why

Nova rarely has only one image to bring in, but the library screen takes one
file per pick. Now that one upload works and a repeat upload finds the
existing Asset, picking several files at once is a small step on the screen
alone.

## What Changes

- The file picker lets Nova choose several PNG or JPEG files in one pick.
- The screen sends the chosen files one at a time, in the order picked, as
  one `POST /api/assets` request each. The server does not change (D1, D2).
- The screen shows one line per chosen file: waiting, then uploading, then
  its result. A saved file, a refusal, a file already in the library and a
  lost answer each keep a line until Nova next picks files (D3, D5, D9). A
  pick of one file is not a special case (D11).
- One file's refusal, 409 or lost answer does not stop the files after it
  (D8). There is no limit on how many files a pick holds (D10).
- Upload stays disabled until every chosen file has an answer (D4). This
  replaces the rule that another file can be chosen at once after a 409.
- The list reloads after each saved file and after each file that got no
  answer. A file found already in the library reloads nothing (D6).
- When the session ends, files not yet sent are not sent (D7).
- Cut from the story: a percentage of bytes sent. Out, as the story says:
  drag and drop, and folder upload (#358).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `image-upload`: the library screen's upload rules change from one file to
  several. "The signed-in library supports one-image upload" becomes an
  upload of one or more files, sent in turn, with a line for each.
  "Upload refusals are clear and recoverable" and "After a repeat upload,
  another file can be chosen at once" change so that the next pick waits
  for the whole pick's answers. The server's rule "Upload accepts exactly one
  file" stays as it is, as do "A lost upload answer says another upload is
  safe" and "Ending the session hides library data": each still holds for
  every file in a pick.

## Impact

- `web/src/Library.tsx`: the picker takes several files, a queue sends them
  in turn, and the single upload state becomes a list of per-file lines.
- `web/src/Library.css`: the per-file lines, from existing tokens.
- `web/src/Library.test.tsx` and `web/e2e/upload.spec.ts`: tests for several
  files, and the tests whose rules change.
- `DESIGN.md`: the Library's upload wording says files, not a file.
- No server, API, database or dependency change.
