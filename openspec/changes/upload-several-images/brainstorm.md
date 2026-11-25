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
| D9 | What does each file's line say? | Waiting: `<name> is waiting.` Uploading: `Uploading <name>…` Saved: `<name> was uploaded.` Refused, already in the library and no answer: the messages the image-upload spec already defines. | User |
| D10 | Is there a limit on how many files Nova can pick at once? | No limit. The files are sent one at a time, so a pick never has more than one upload open. Each saved file still reloads the whole list (D6), so a pick of n saved files can send up to n list requests. That cost is accepted and recorded in design.md. | User |
| D11 | Is one picked file a special case? | No. One file is a pick of one: it shows one line, which stays until the next pick, as D5 says. | User |
| D12 | If Nova presses Sign out while files are still waiting, when does the screen stop sending them? | Once the screen knows the session has ended: the sign-out has answered, an upload or list request has answered 401, or the events connection says so. A file sent while the sign-out is still pending gets the answer the server gives when the request arrives: its usual answer while the session is still valid, or 401 once it is gone, and the screen then reports the lost session. A sign-out that fails or gets no answer tells the screen nothing about the session, so the pick goes on. | User |
| D13 | Must every saved file or lost answer cause its own reload of the list? | No. The list reloads after a saved file or a lost answer, and answers that come back before that reload starts may share it. | User |
| D14 | How does the screen stop at once when it learns the session has ended? | `Access` keeps an abort signal for the signed-in view and passes it to `Library`. It aborts the signal in the same call that learns the session ended (the sign-out answered 204 or 401, or `sessionLost` ran for a library request or the events connection), before it changes the view. The upload loop checks the signal before each send. | User |

## Changes from the story

- Cut: a percentage of bytes sent for each file. "Progress" means each file's state (D3).
- Out, as the story says: drag and drop, and folder upload (#358).

D12 and D13, and the corrected reason in D10, are the user's answers to the plan review's first round. D14 is the user's answer to its second round. That round also corrected D12's wording: the server checks the session once, when a request arrives, so a file it admits gets its usual answer, not only "saved" or 401.

Confirmed by user: yes
