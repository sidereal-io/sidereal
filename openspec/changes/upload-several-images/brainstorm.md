## Story

#531 upload-several-images — Nova uploads several images at once and sees progress for each.
https://github.com/sidereal-io/sidereal/issues/531

## Decisions

| # | Question | Answer | Source |
|---|---|---|---|
| D1 | Does the server take several files in one request, or does the screen send one request per file? | One request per file. `POST /api/assets` keeps the rule "Upload accepts exactly one file". A multi-file endpoint can come later. | User |
| D2 | Are the chosen files sent one at a time or all at once? | One at a time, in the order picked. Files not yet sent show as waiting. | User |
| D3 | What does each file's progress show? | Its state only: waiting, uploading, then its result. No percentage of bytes sent, so the upload stays on `fetch`. | User |
| D4 | Can Nova pick more files while some are still uploading? | No. Upload stays disabled until every chosen file has an answer. So after a 409, the next file can be chosen only once the whole pick has answers, not at once as "After a repeat upload, another file can be chosen at once" says today. | User |
| D5 | How long does each file's result stay on screen? | Until Nova next picks files. The new pick replaces the list of results. | User |
| D6 | When does the library list reload? | After each saved file, and after each file that got no answer. A file found already in the library reloads nothing. | User |
| D7 | If the session ends while files are still waiting, what happens to them? | They are not sent. The library and results are hidden, as "Ending the session hides library data" requires. | User |
| D8 | Does one file's failure stop the rest? | No. A refusal, a 409 or a lost answer for one file leaves the next files to be sent as normal. | Evidence: #531 packet, Scope |
| D9 | What does each file's line say? | Waiting: `<name> is waiting.` Uploading: `Uploading <name>…` Saved: `<name> was uploaded.` Refused, already in the library and no answer: the messages the image-upload spec already defines. | Assumption |
| D10 | Is there a limit on how many files Nova can pick at once? | No limit. The files are sent one at a time, so a large pick adds no load beyond one upload. | Assumption |
| D11 | Is one picked file a special case? | No. One file is a pick of one: it shows one line, which stays until the next pick, as D5 says. | Assumption |

## Changes from the story

- Cut: a percentage of bytes sent for each file. "Progress" means each file's state (D3).
- Out, as the story says: drag and drop, and folder upload (#358).

Confirmed by user: no
