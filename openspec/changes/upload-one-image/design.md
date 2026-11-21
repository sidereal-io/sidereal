## Context

`plugin-abi/src/lib.rs` has ID-only Source, Operator and Sink traits and a
Registry with registration and ID listing. `packs/astro` registers a stub
FITS Source. `core/src/storage.rs` validates and pairs a root but has no
object store. Migrations and normalized schema snapshots stop at version 3.
The startup validator compares the full schema with those snapshots.

The server already guards every `/api` route through
`server/src/access/mod.rs`; routes must be added before its `route_layer`
calls. `main.rs` validates storage but does not pass it to HTTP services.
`web/src/Access.tsx` owns access state and the events socket. Its signed-in
branch currently shows the username and Sign out button. `App.tsx` owns the
health display. There is no client-side page router to extend.

The authoritative vocabulary is `openspec/glossary.md`. No affected capability
has a narrower glossary. The confirmed brainstorm and specs fix the behavior.

## Goals / Non-Goals

**Goals:** execute the built-in Source through the public contract; keep format
knowledge in the image pack; make committed originals durable; reuse current
session, database, storage-pairing and UI seams.

**Non-Goals:** freeze the plugin contract, implement Operator execution or
facets, install plugins, add processing goals, deduplicate logical Assets,
sweep abandoned files, reclaim objects, decode images on upload, or introduce
a new UI framework. The Proposed scripting ADR is not a prerequisite for this
built-in Rust Source.

## Test Seams

- **Primary test seam:** server HTTP integration tests with a real throwaway
  PostgreSQL database and storage directory. Extend the patterns in
  `server/tests/access.rs` and `server/tests/storage.rs` into `tests/assets.rs`.
  Drive claim/sign-in, upload, list and original reads through production routes.
  The process harness proves persistence with a new server process.
- **Existing seams reused:** core storage unit tests for deterministic file,
  directory-sync and commit failure injection; plugin-abi/pack registration
  tests for dispatch; React Testing Library in `web/src/Access.test.tsx` and
  a new `Library.test.tsx` for selection, upload, display error and session loss.
  Reuse `TestDatabase` from core's database test support. Run database tests
  with `TEST_DATABASE_URL` pointing to the repository fixture's test role.
- One end-to-end browser scenario in `web/e2e/upload.spec.ts` demonstrates
  sign-in, upload and actual image rendering against disposable data. Reuse
  the existing Playwright runner; require isolated database/storage and fresh
  server processes so it cannot alter a contributor's library. The Rust process
  test owns restart and byte-equality assertions, not a browser mock.

## Scrutiny Tier & Blast Radius

- **Tier:** high
- **Blast radius:** the public Source contract, both first-party packs, core
  persistence and filesystem boundaries, server composition and guarded API,
  and the signed-in UI. Durable data and access checks are involved.

## Decisions

### 1. Extend the existing Source contract with a narrow execution context

Add an object-safe `Source::run` and lookup by stable Source id in Registry.
For this synchronous byte-recognition slice, execute the Source on a blocking
worker after staging completes. A run-scoped `AssetContext` in `plugin-abi`
provides bounded reads of that candidate, one `propose_asset` operation,
logging and cancellation observation. The implementation rejects a second
proposal. Source completion returns success or a typed refusal/failure; a
proposal is pending data until execution and request validation both succeed.

An Asset proposal carries only kind and recognized format/media type. The
host supplies the display name from the request, mints ids, hashes bytes and
records the actual Source id and implementation version plus ingestion time
as provenance. The context exposes no filesystem path, pool, trusted hash or
identity setter. This is the first usable subset of ADR-001's Asset context,
not the later Operator/facet API. Check cancellation while staging and before
publication; once database commit succeeds, a disconnected client does not
undo the Asset.

Create `sidereal-pack-image` under `server/crates/packs/image`, with
`ImagePack::register` registering `image.manual-upload`. The pack recognizes
the eight-byte PNG signature or JPEG's leading `FF D8 FF` bytes and proposes
kind `image` with `image/png` or `image/jpeg`. It does not decode. Its project
crate dependency is only `sidereal-plugin-abi`; no image decoder is needed.
The server constructs the Registry and registers AstroPack and ImagePack.
Update the astro Source stub to explicitly report that execution is not yet
supported. Bump `ABI_VERSION` because implementers must add execution methods;
retain the intentionally unfrozen contract and document that change.

Alternative: putting a PNG/JPEG switch in core or the HTTP handler is shorter
but bypasses the agreed plugin seam. Building the full future Operator context
now would add unused authority and delay this slice. The narrow interface is
expected to evolve before the ABI freezes.

Extend `server/scripts/check-arch.sh` to check every workspace pack's dependency
tree for core, server and other packs, rather than only checking astro for
core. Keep pack registration tests and an architecture-lint failure fixture
that demonstrates a forbidden dependency is detected.

### 2. Persist the Asset/version model already selected by ADR-003

Add the next forward-only migration, `4_assets.sql`, with `assets` and
`asset_versions`. Assets hold UUID `id`, text `kind`, text `name`, UUID
`current_version_id`, and `revision` initialized to zero. Versions hold UUID
`id`, UUID `asset_id`, a 64-character lowercase BLAKE3 hash, nonnegative byte
size, format/media type, Source id/version and ingestion timestamp. Keep the
hash indexed and non-unique. The current-version reference must belong to the
same Asset: use a composite foreign key to `(asset_id, id)`, deferred until
transaction commit, so neither an absent version nor another Asset's version
can become current. Insert both records in one transaction after publication.

Core exposes immutable version reads and initial Asset creation, with no
version update or deletion operation. No general metadata columns or facet
tables are added. Regenerate `core/schema/4.json` through `mise run db-schema`
and add the version to startup's expected-schema selection. Tests cover a
fresh database, upgrade from migration 3, and rejection of inconsistent
current-version references. The existing migration checks must still reject
a newer schema with an older binary.

Alternative: a single table would erase the identity/version distinction.
Unique hashes on versions would prevent multiple versions sharing bytes.
Both alternatives contradict accepted decisions; no new data-model fork is
introduced here. This slice does not promise the later lookup of an existing
Asset on repeated bytes; storage sharing is independent of that lookup.

### 3. Stage a bounded stream and publish without replacing objects

Add an object-store service beside the existing StorageRoot pairing code,
constructed only from the root that passed startup pairing and probes.
Create `tmp/` and `store/` lazily at first ingest, preserving the existing
marker-only first-start scenario. Resolve operations relative to held root
and directory handles, with no-follow directory traversal and regular-file
checks. Generate staging UUIDs exclusively; request filenames never become
path components. Do not rely on a canonicalize-then-open check with a race.
The existing libc dependency supports descriptor-relative operations.

The HTTP adapter streams one multipart field to core staging in bounded
chunks and counts actual file bytes. Do not buffer a 100 MiB request in RAM.
After the field completes, finish parsing the entire multipart request and
refuse any extra field or malformed trailing data before executing or committing
an upload. Set the route's transport limit above 100 MiB to allow bounded
multipart overhead (1 MiB); enforce the exact 100 MiB file cap separately.
An overlong or malformed multipart envelope gets a clear client error. Leave
the body limits of access endpoints unchanged.

After staging, core reads the candidate to calculate BLAKE3 and its byte size,
and the selected Source reads its signature through the context. Flush and
sync the accepted file before publication. Use an atomic rename that refuses
to replace an existing entry, with a platform-supported no-replace operation;
an unsupported primitive fails closed. A plain overwriting rename or
exists-then-rename sequence is insufficient. On a concurrent existing-object
result, open the winner without following links, check that it is regular and
has the expected size, and sync it before adoption. An unexpected size is a
storage failure, not permission to overwrite. Full integrity verification of
preexisting objects remains the later verify/repair feature.

Sync the leaf hash directory and each new ancestor entry, through `store/`
and the root, before committing. This includes the existing-object race:
ensure the adopted object and its directory chain are durable even if the
winning request has not yet finished syncing. Reject a filesystem boundary
between staging and store; do not fall back to a non-atomic cross-device copy.
Only then insert the Asset and version through core's database service.

Normally remove this request's staging file on refusal or reuse. A crash can
leave a staging file, or an unreferenced published object if it happened after
rename but before database commit. Neither appears in the library. Do not
sweep startup leftovers or reclaim objects; those are explicitly deferred.
Never remove a published object on database failure, because another version
can already reference it. Directory-sync faults are tested with a narrow
injected filesystem seam, as marker publication already does.

Alternative: committing first risks a row pointing to missing bytes. Copying
straight into the final hash path exposes partial content. The accepted
ADR-011 ordering excludes both.

### 4. Wire guarded APIs around core services

Add `server/src/assets.rs` and core asset persistence/ingest modules. Carry an
optional object-store service and the Registry from main into HTTP composition;
keep the existing database-only test constructors usable. Add the Asset routes
inside the protected router before session and origin layers. Do not mount
`STORAGE_ROOT` with ServeDir or add an authentication exemption.

The API uses these shapes:

- `POST /api/assets`: multipart field `file`; 201 `{ "asset": <entry> }`.
- `GET /api/assets`: 200 `{ "assets": [<entry>, ...] }`, with entries ordered
  by ingestion time and id for stable rendering in this slice.
- `<entry>`: `id`, `kind`, `name`, `current_version_id`, `original_url`.
- `GET /api/assets/{id}/original`: stream the file resolved from the persisted
  current version's hash, with the pack-recognized Content-Type, nosniff and
  no-store headers. Invalid or unknown ids get 404.

Use fixed JSON error messages for 400 malformed upload, 413 size limit,
415 unsupported format and 503 persistence/storage failures. Keep guard-owned
401/403 behavior unchanged. Refusals reveal neither filesystem paths nor
nested SQL errors to HTTP clients; operational logs can identify escaped paths
without credentials. Require a nonempty filename; treat it only as display
text and never echo it into headers. Do not auto-retry POST: a lost response
after commit is ambiguous until logical deduplication is added.

Alternative: a separate unguarded file-serving route would weaken the existing
access boundary. Loading whole originals before sending them would multiply
memory use. Core-backed streaming avoids both.

### 5. Add the library inside the signed-in view

Render a new `Library` component from Access's signed-in branch. Keep App's
health display and Access's credentials and socket transitions intact. Use a
labelled single-file input accepting PNG/JPEG and a type=button Upload action,
without adding a credentials-style form. Mounting loads the list; success
reloads it; list errors have Retry; upload errors retain the selected file.
Client size/type hints do not replace server validation. On 401, notify Access
to return to sign-in. Unmount aborts outstanding fetches and ignores late
responses. Image load errors retain the name and show the fixed display message.

Use a gallery-family layout: uncropped originals at their natural ratio,
square corners, filename in the mono face, a primary violet Upload control,
and quiet sign-out/retry controls. Display status colors only in badges.
Use existing spacing, type, color and radius tokens. Do not add thumbnails,
preview generation, image transforms, zoom, sorting controls or a client router.

**New semantic or component tokens: none.** Existing primitives cover this
slice; DESIGN.md permits direct primitive use until a semantic role exists.
Before building the new visual components, use `visual-options` as DESIGN.md
requires, record the selected component treatment there, and verify it uses
these existing tokens. That visual comparison can adjust presentation within
this plan; a choice that changes scope or requires new token values returns
to planning rather than silently expanding this task.

Alternative: a separate asset page and viewer would introduce the later
viewing story early. Keeping the library inside the current access lifecycle
also makes session loss hide its data immediately.

## Risks / Trade-offs

- [Recognized damaged bytes can be accepted] → Show the explicit browser
  display error; full decoding remains outside the confirmed upload boundary.
- [No-clobber rename or directory sync differs by operating system] → Test
  the supported platforms and refuse unsafe publication; never silently weaken
  durability to make an unsupported filesystem appear to work.
- [A crash after publication leaves an unreferenced object] → Keep it out of
  listing and retain it for safe reuse; never delete shared content on failure.
- [The whole-library list and original-sized images cost bandwidth] → This is
  the one-image slice; previews and richer browsing remain separate changes.
- [The response can be lost after commit] → Do not retry automatically or
  promise logical deduplication before the follow-up story implements it.
- [A trusted in-process pack is not an OS sandbox] → Preserve the public
  mediated contract and dependency checks; do not advertise untrusted plugin
  installation or the Proposed scripting engine.

## Migration & Rollback

Deploy the server and built web together. Startup applies migration 4 before
serving HTTP and retains the existing storage UUID pairing. No new environment
variable is required. `tmp/` and `store/` appear on first upload, not at pairing.

Migrations remain forward-only. An old binary must refuse the newer schema.
Prefer a forward fix. To restore a pre-change installation, stop the server
and restore its matched database and storage backup; reverting only the code
or deleting the new tables is not a supported rollback. Do not touch the
maintenance branch or its release process.
