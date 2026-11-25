## Context

All of this change is in the library screen, `web/src/Library.tsx`. Today:

- The hidden file input has no `multiple` attribute. Its `onChange` handler
  takes `files[0]`, clears the input's value so the same file can be picked
  again, and calls `send(file)`.
- `send` sends the file with `sendFile`, which posts one `file` field with
  `fetch` and returns one of `"saved"`, `"unconfirmed"`, `{ refused }`,
  `{ found }` or `sessionEnded`. `sendFile` is never retried.
- One `Upload` state holds what the screen shows: `idle`, `sending`,
  `refused`, `found` or `unconfirmed`. A saved file goes back to `idle`, so
  its line disappears.
- `sendingRef` blocks a second choice while a request is open. The Upload
  button is disabled while the state is `sending`.
- `uploadsRef` holds an `AbortController` that is aborted when the Library
  unmounts. `Access.tsx` learns that the session has ended when the sign-out
  answers 204 or 401 (`signOut`), or when `sessionLost` runs for a library
  request or the events connection. It then sets the signed-out view, but
  React unmounts the Library, and runs its cleanup, only at the next commit.
  An upload answer that arrives in that gap would let the loop send the next
  file after the screen knows, which D12 forbids. So the unmount signal alone
  is too late, and D14 adds a signal that `Access` aborts at once.
- `Access` keeps the Library mounted while the sign-out request is pending,
  and after a sign-out that fails or gets no answer, which shows the notice
  "Something went wrong" in the Library.
- A saved file or a lost answer bumps `generation`, which loads the list
  again. The list effect aborts the previous list request on each bump.

The server keeps one file per request (D1), so nothing changes outside
`web/` and `DESIGN.md`. In `web/`, the change is in `Library.tsx`, plus the
session signal in `Access.tsx` (D14) and the check that a sign-out answer
still belongs to the current view (D15).

## Goals / Non-Goals

**Goals:**

- Turn the single upload state into one line per picked file, driven by one
  loop that sends the files in turn.
- Keep `sendFile`, `refusal` and the existing message texts as they are, so
  each file's answer reads exactly as a single upload's does today.

**Non-Goals:**

- No new endpoint, no change to `POST /api/assets` and no server test.
- No byte progress, so no switch from `fetch` to `XMLHttpRequest` (D3).
- No cancel button for a pick in progress, and no picking while a pick runs
  (D4).
- No live region or other new accessibility behavior. The lines stay plain
  text, as the status lines are today.

## Test Seams

- **Primary test seam:** the component harness in `web/src/Library.test.tsx`.
  It renders `Library` with `fetch` stubbed, fires a `change` event on the
  file input, and reads the lines, buttons and recorded requests. Every
  requirement in the delta spec can be checked there.
- **Existing seams reused:** `stubServer` and `deferred` in
  `Library.test.tsx`. `stubServer` gains one way to say "this turn gets no
  answer", so one list of answers can mix 201, 409, 415 and a lost answer
  in the order the files are sent. The `Library` tests pass their own
  `session` signal and abort it to stand in for `Access`. The sign-out and
  the events connection during a pick are tested through `Access` in
  `web/src/Access.test.tsx`, because only `Access` aborts the real signal: it
  renders `Access` with a stubbed `fetch` whose upload and
  `DELETE /api/session` answers the test releases, and the events socket
  stub the file already uses. Where an upload answer must reach the loop
  before React commits the view change, the test ends the session and
  releases the answer inside one `act` scope, so the Library's unmount
  signal alone cannot make the test pass. The end-to-end
  journey in
  `web/e2e/upload.spec.ts` picks its two fixture images in one pick, which
  proves the browser's `multiple` picker reaches the same code.

## Scrutiny Tier & Blast Radius

- **Tier:** low
- **Blast radius:** the library screen in `web/` (`Library.tsx`,
  `Library.css` and their tests), the session signal in `Access.tsx` and its
  new tests in `Access.test.tsx`, the upload end-to-end test, and the
  Library section of `DESIGN.md`. In `Access.tsx`, `signOut` also ignores
  an answer that arrives after its signed-in view has ended. No API,
  schema, server, session or authorization code changes.

## Decisions

### One list of lines replaces the single upload state

State becomes `lines: Line[]`, one per picked file, in pick order:

```ts
type Line =
  | { name: string; kind: "waiting" | "sending" | "saved" | "unconfirmed" }
  | { name: string; kind: "refused"; reason: string }
  | { name: string; kind: "found"; existing: string | null };
```

A new pick sets `lines` to one `waiting` line per file, which drops the last
pick's lines (D5). Each answer replaces its own line by index. The screen
renders the lines in a `<ul aria-label="Uploads">`, one `<li>` per line,
keyed by index: the list is only ever replaced whole, so the index is
stable within a pick.

Alternative: keep one `Upload` state and add a queue beside it. Rejected:
two sources of truth for what the screen shows, and the results of earlier
files would have nowhere to live.

### One loop sends the files in turn

`send(files: File[])` replaces `send(file)`. Its signal is
`AbortSignal.any([session, uploads])`: the `session` signal from `Access`
(see the next decision) and the Library's own unmount signal. It sets
`sendingRef`, writes the waiting lines, then for each file:

1. Stops if the signal is aborted.
2. Marks the file's line `sending` and awaits `sendFile`.
3. Stops if the signal is aborted, without touching state.
4. On `sessionEnded`, calls `onSessionLost()` and stops: the files after it
   are never sent (D7).
5. Otherwise writes the file's result line, and bumps `generation` for
   `"saved"` and `"unconfirmed"` only (D6). A refusal or a 409 moves on to
   the next file (D8).

After the last file, it clears `sendingRef`. A sequential `for … of` with
`await` gives the one-at-a-time order (D2) without a queue object.

While a sign-out is pending the loop goes on (D12). A file sent then reaches
the server's session guard
([ADR-015](../../../docs/decisions/ADR-015-opaque-database-sessions.md)),
which looks the session up once, when the request arrives. An admitted file
gets its usual answer. A file that arrives after the session is gone gets
401, which takes step 4. When the sign-out then answers, the abort also ends
the request in flight, and step 3 drops its result. No sign-out state is
needed in `Library`.

Alternative: send all files at once with `Promise.all`. Rejected by D2.

### `Access` aborts a session signal as soon as it learns the session ended

`Access` keeps one `AbortController` for each signed-in view, in a ref. It
makes a new one each time it shows the signed-in view, from the first
session check or from a sign-in, and passes its signal to `Library` as the
required prop `session`. It aborts it in the same call that learns the
session ended, before it sets the signed-out view:

- in `signOut`, when the answer is 204 or 401 and the view that sent the
  sign-out is still current;
- in `sessionLost`, which the events connection and the Library's own 401
  answers call.

A sign-out that answers anything else, or no answer, aborts nothing (D12).

`signOut` reads the ref's controller when it starts, before it sends the
`DELETE`, and keeps it. When the answer comes, it acts only if that
controller is still the one in the ref and is not aborted. Otherwise the
view the sign-out came from has already ended, for example because the
events connection ended the session and Nova then signed in again. Then it
ignores the answer: it aborts nothing, sets no view and shows no notice
(D15). A 204 or 401 that arrives late therefore cannot stop the next view's
pick or send Nova back to the sign-in form, and a late 500 cannot put
"Something went wrong" on the next view. Today `signOut` applies its answer
with no such check (`web/src/Access.tsx:185-191`); this change adds it.

The check alone does not protect the next sign-in's cookie. The server's
204, and the 401 for a token no row backs, both carry a `Set-Cookie` that
clears the session cookie, and the browser applies it whatever the script
does with the answer. So `signOut` sends the `DELETE` with the kept
controller's signal. When the view ends, the abort that stops the pick also
cancels the request, and an answer that has not arrived by then sets no
cookie.

The abort runs in the same promise callback that reads the answer, so it
comes before any render. The loop's next check (step 1 or step 3) then sees
it, whether or not React has unmounted the Library yet (D14). The Library's
unmount signal stays, for an unmount that does not come from a session end.

The list effect keeps its own controller: it already drops a late list
answer, and the session end hides the list.

Alternative: abort the Library's signal only in its unmount cleanup, as
today. Rejected: React runs that cleanup at the commit after the view
changes, and an upload answer can arrive before it (see Context).

Alternative: abort only the captured controller, but still set the
signed-out view on a late answer, as today, and file the stale view change
as a separate bug. Rejected (D15): Nova would still be sent to the sign-in
form in the middle of a new pick.

### The list reloads after a saved file or a lost answer

Step 5 bumps `generation`, and the list effect loads the list once for each
`generation` value that React commits. React can batch two bumps that come
in the same task into one render, so two answers that come back together
can share one load. The spec allows this (D13): it asks for a load that
starts after each saved file or lost answer, not one load per answer. So no
counter or forced load per answer is added. The tests release answers one
at a time with `deferred`. After a saved file or a lost answer, a test waits
until the stub has recorded the list request it causes before it releases
the next answer. The list effect starts that request after the commit, so
the file's line can show before it starts. After a 409, the test waits for
the line. So each counted load belongs to one answer.

### Upload is disabled while any line is waiting or sending

The button's `disabled` is derived from `lines`: true while any line is
`waiting` or `sending`. `sendingRef` stays as the synchronous guard, so a
second `change` event in the same tick sends nothing (D4). No separate busy
state is kept.

### The picker takes several files

The input gets `multiple`. The `onChange` handler copies
`event.currentTarget.files` into an array before it clears the value, since
clearing the value can empty the file list it read. A cancelled picker,
which gives no files, is not a pick: it sends nothing and keeps the last
lines.

### A saved file keeps a line

`"saved"` now writes `<name> was uploaded.` instead of going back to `idle`
(D9, D11). The other line texts are the ones `Library.tsx` shows today.

### The lines use existing tokens

Each `<li>` uses the existing `library-status` class. The `<ul>` gets a new
`library-uploads` class that removes the list's bullets, margin and padding
and sets a `spacing-8` gap between lines. DESIGN.md names `spacing-8` for
"small gaps inside controls and lists".

Tokens added: none.

### DESIGN.md says files

The Library section's "Upload is one primary button" line changes from "a
file" to "files", and the "Every other state" line names each picked
file's line. This is wording, not a token, so `pnpm tokens` has nothing to
regenerate.

### Records

No decision here is costly to reverse: the server contract is unchanged and
the screen can change again freely. No ADR is needed.

## Risks / Trade-offs

- [A long pick makes a long list of lines above the library, and there is
  no limit (D10).] → Accepted. Each line is one short quiet line, and the
  next pick clears them.
- [Upload stays disabled for the whole of a long pick (D4).] → Accepted for
  now. Each line shows which file is on its way and how many wait.
- [Each saved file reloads the whole list, and the list has no paging
  (D6, D10).] → Accepted. A pick of n files that are all saved can start up
  to n list requests, and each one returns the whole, growing library. The
  list effect aborts the browser's previous list request on each bump, but
  the server may already be answering it, so the server's work is not
  saved. Answers that come back together may share one load (D13), which
  lowers the count only by chance. Paging the list, or one load at the end
  of a pick, is for a later change.
- [A list reload after each saved file could race the next file's answer.]
  → The list effect already aborts its previous request on each
  `generation` bump, so only the newest list shows.
- [A state write after the session ends could show old lines again.] → The
  loop checks the aborted signal before each send and after each answer,
  as `send` does today.
- [`AbortSignal.any` is newer than `AbortController`.] → It is in every
  current browser, and jsdom has had it since version 22.1 (`web/` uses
  jsdom 31). If a test shows it missing, the loop can check `session.aborted || uploads.aborted` instead,
  and pass the unmount signal to `fetch`.

## Migration & Rollback

N/A. Nothing changes shape. Rolling back is reverting the commits.
