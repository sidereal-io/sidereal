## Story

[#521 — upload-one-image — Nova uploads a PNG or JPEG and sees it in the library, and it is still there after a restart](https://github.com/sidereal-io/sidereal/issues/521)

This is the first working upload path in epic #356. Nova needs to bring one
image into the library and see it; Sam needs its original bytes to survive a
server restart. The plan uses the `lets-standard` schema because this is a
Story of Size M.

## Decisions

| # | Question | Answer | Source |
|---|---|---|---|
| D1 | Where do the manual-upload Source and PNG/JPEG handling live? | Create the first-party `server/crates/packs/image` pack now. Its only dependency on another project crate is `plugin-abi`; the server registers it through `Pack::register`. Put the manual-upload Source and PNG/JPEG handling there. The image Operator joins this pack in a later story. | User |
| D2 | What is the maximum size of one uploaded file? | Accept files up to and including 100 MiB (104,857,600 bytes). Reject larger files with a clear message and create no Asset. Configurable limits are deferred. | User |
| D3 | Does upload require a complete image decode? | Recognize PNG or JPEG from the bytes, following ADR-011's format-sniffing scope, without requiring full decoding. Reject an unrecognized format before creating an Asset. If an accepted image cannot render, show a clear display error. | User |
| D4 | What must Nova be able to demonstrate? | Sign in, choose one PNG or JPEG from the computer, upload it, and see the image listed by name and displayed in the library. After a server restart, the same Asset and original bytes remain available. | Evidence: #521 story packet |
| D5 | What durable records does this slice introduce? | Add Asset and immutable AssetVersion records through a forward-only migration. An Asset has an opaque id, kind, display name and current-version reference; its version has an opaque id, BLAKE3 hash, byte size, format and provenance. Core owns identity and hashing. | Evidence: #521 story packet; docs/decisions/ADR-003-asset-identity-and-content-revisions.md; openspec/glossary.md |
| D6 | How are original bytes protected? | Follow ADR-011: copy into `tmp/{uuid}`, hash and inspect the file, sync it, publish it into `store/{h[0:2]}/{h[2:4]}/{h}`, sync the store directory and commit database records last. Share an existing object rather than copying it again. Reject path traversal and symlink escape; do not reclaim stored objects. | Evidence: #521 story packet; docs/decisions/ADR-011-storage-tree-layout.md |
| D7 | Which interfaces belong to this slice, and who can use them? | Execute manual upload through the public Source contract and Registry. Provide `POST /api/assets`, `GET /api/assets` and access to original bytes behind the existing administrator session guard; uploads also retain the existing origin checks. | Evidence: #521 story packet; openspec/specs/instance-access/spec.md; server/crates/server/src/access/mod.rs |
| D8 | What remains outside this slice? | Finding an existing Asset for repeated bytes belongs to #522; startup temporary-file cleanup to #523; metadata facets and processing to #524; TIFF and previews to #526. This story does not add the later Operator, viewer pan/zoom, multi-file upload, drag-and-drop, deletion or moves. Sharing an existing stored object is still required by D6. | Evidence: #521 story packet; #356 scope and shared decisions |
| D9 | Which existing storage and UI constraints still apply? | Preserve storage/database pairing and database-only health serving when `STORAGE_ROOT` is unset. Keep access and session behavior intact. Build the library UI using DESIGN.md tokens and rules. | Evidence: openspec/specs/managed-storage/spec.md; openspec/specs/instance-access/spec.md; DESIGN.md |
| D10 | Does `mise run e2e` run its own stack? | Yes, always. It makes a new database through the test role, a temporary `STORAGE_ROOT`, and a fresh server and Vite on their own ports. It never reuses a running `mise run dev`, and drops everything afterward. It refuses to run if it cannot create them. The ci and dev-commands specs change to match. | User (plan review round 1, finding 4) |
| D11 | What happens when `STORAGE_ROOT` and `WEB_ROOT` overlap? | The server refuses to start when they are the same folder, or one holds the other, after following symbolic links. The message names both settings. A symbolic link someone writes into `WEB_ROOT` stays covered by the existing `docs/install.md` warning. | User (plan review round 1, finding 1) |
| D12 | Is an existing stored object checked before reuse? | Yes. Upload re-hashes it before reuse. On a mismatch it answers 503, commits nothing, and leaves the file unchanged. | User (plan review round 1, finding 3) |
| D13 | Does the Library add semantic tokens? | Yes: `color-action-primary`, `color-text-on-action`, `color-text-body`, `color-text-secondary`, `color-border-hairline` and `color-focus-ring`, each pointing at an existing primitive. Access's Sign-in button switches to `color-action-primary`. Edit DESIGN.md first, then run `pnpm tokens` in the same change. | User (plan review round 1, finding 8) |
| D14 | What does uploading the same bytes again do in this change? | Leave it unspecified. #522 owns what uploading the same bytes again does. Add no requirement, scenario or temporary exception for it here; the design's non-goals name #522 as the owner. | User (plan review round 1, finding 9) |
| D15 | What about a bind mount that puts `STORAGE_ROOT` inside `WEB_ROOT`? | Treat it like a symbolic link written into `WEB_ROOT`. Keep D11's startup check. `docs/install.md` warns that no mount or link may expose `STORAGE_ROOT` inside `WEB_ROOT`, and the spec names the case as outside the check. | User (plan review round 2, finding 1) |
| D16 | Does fetching an original re-hash its bytes? | No. The spec says retrieval returns the bytes stored at the version's hash path. Detecting a damaged or replaced file belongs to the later verify-and-repair feature. A missing or unreadable file still gets 503. | User (plan review round 2, finding 9) |
| D17 | How do the browser tests share the one throwaway stack? | The e2e suite runs with one worker everywhere. The upload journey runs serially with no retries. | User (plan review round 2, finding 5) |
| D18 | Which `WEB_ROOT` does the file server use? | Startup resolves `WEB_ROOT` once, with symbolic links followed, and uses that one folder for the overlap check, the `index.html` check and the file server. A `WEB_ROOT` link that is repointed after startup takes effect only after a restart. | User (plan review round 3, finding 1) |
| D19 | Where does an upload's multipart data end? | At the closing delimiter, which ends validation. Bytes after it are an epilogue (RFC 2046) and are ignored. A body that ends or fails before the closing delimiter gets 400 and commits nothing. | User (plan review round 3, finding 2) |

## Changes from the story

- D1 resolves the packet's open question about the Source's home. It creates
  `packs/image` in this story so the later image Operator can use the same pack,
  as epic #356 already requires.
- D2 adds a fixed 100 MiB file limit to make refusal behavior explicit.
- D3 makes the validation boundary explicit: byte recognition at upload and a
  visible error when the browser cannot display an accepted image.
- No work is split into a new issue. The existing follow-up stories retain
  their scope.

Confirmed by user: yes
