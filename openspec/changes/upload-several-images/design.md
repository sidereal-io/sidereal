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
  unmounts. `Access.tsx` unmounts the Library once it knows the session has
  ended: the sign-out has answered 204 or 401, a library request has called
  `onSessionLost`, or the events connection has reported the end. It keeps
  the Library mounted while the sign-out request is pending, and after a
  sign-out that fails. So an aborted signal means "the screen knows the
  session ended", which is the moment D12 names.
- A saved file or a lost answer bumps `generation`, which loads the list
  again. The list effect aborts the previous list request on each bump.

The server keeps one file per request (D1), so nothing changes outside
`web/` and `DESIGN.md`.

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
  in the order the files are sent. The sign-out during a pick is tested
  through `Access` in `web/src/Access.test.tsx`, because only `Access` holds
  the pending sign-out: it renders `Access` with a stubbed `fetch` whose
  upload and `DELETE /api/session` answers the test releases. The end-to-end
  journey in
  `web/e2e/upload.spec.ts` picks its two fixture images in one pick, which
  proves the browser's `multiple` picker reaches the same code.

## Scrutiny Tier & Blast Radius

- **Tier:** low
- **Blast radius:** the library screen in `web/` (`Library.tsx`,
  `Library.css` and their tests), one new test in `Access.test.tsx`, the
  upload end-to-end test, and the
  Library section of `DESIGN.md`. No API, schema, server, session or
  authorization code changes.

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

`send(files: File[])` replaces `send(file)`. It sets `sendingRef`, writes the
waiting lines, then for each file:

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

The loop stops sending exactly when the signal is aborted, which is when
`Access` unmounts the Library (see Context). While a sign-out is pending the
loop goes on (D12). A file sent then reaches the server's session guard
([ADR-015](../../../docs/decisions/ADR-015-opaque-database-sessions.md)),
which either saves it under the still-valid session or answers 401. A 401
takes step 4. When the sign-out then answers, the abort also ends the
request in flight, and step 3 drops its result. No new sign-out state is
needed in `Library`.

Alternative: send all files at once with `Promise.all`. Rejected by D2.

### The list reloads after a saved file or a lost answer

Step 5 bumps `generation`, and the list effect loads the list once for each
`generation` value that React commits. React can batch two bumps that come
in the same task into one render, so two answers that come back together
can share one load. The spec allows this (D13): it asks for a load that
starts after each saved file or lost answer, not one load per answer. So no
counter or forced load per answer is added. The tests release answers one
at a time with `deferred`, and wait for each answer's line before the next,
so each counted load belongs to one answer.

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

## Migration & Rollback

N/A. Nothing changes shape. Rolling back is reverting the commits.
