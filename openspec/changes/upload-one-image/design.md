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

What uploading the same bytes again does to the library is not specified by
this change. Story #522 owns it: whether a repeat upload finds the existing
Asset, and what the response says. This change specifies only that matching
bytes share one stored object (D6, D8, D14). It adds no requirement, scenario
or interim exception for repeat uploads, and tests must not assert a repeat
upload's Asset outcome.

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
  claim, sign-in, upload, actual image rendering, and the library after a
  server restart. The Rust process test owns byte-equality and identity
  assertions across restart; the browser test asserts what Nova sees.
- **Isolated browser stack (D10).** `mise run e2e` always runs its own
  throwaway stack and never reuses a running `mise run dev`:
  - `scripts/e2e.sh` creates a new database named `sidereal_test_e2e_<random>`
    through the fixture's test role, with new subcommands of
    `server/scripts/db-fixture.sh` that run `psql` inside the fixture
    container, as `orphan-clean` already does. The `sidereal_test_` prefix
    means `orphan-clean` also removes a database a killed run leaves behind.
  - It creates a temporary `STORAGE_ROOT` with `mktemp -d`.
  - It ignores an inherited `DATABASE_URL` and `STORAGE_ROOT`, so it can never
    point the server at a contributor's data.
  - It picks a server port and a Vite port from the checkout, the way
    `db-fixture.sh` picks the database port, so neither is 5000 or 5173 and
    two checkouts do not collide. It passes them to Playwright through
    `SIDEREAL_E2E_SERVER_PORT` and `SIDEREAL_E2E_WEB_PORT`.
  - The script, not Playwright's `webServer`, starts the built server in the
    background with `PORT`, `DATABASE_URL` and `STORAGE_ROOT` set, keeps its
    process id in the run's temporary folder, and waits for `/healthz`. This
    lets the browser test restart it. The test calls a restart helper that the
    script provides; the helper stops that process id and starts the server
    again with the same settings.
  - `web/playwright.config.ts` reads the two port settings, starts only Vite,
    with `reuseExistingServer: false`, `--port <web port> --strictPort` and
    `SIDEREAL_DEV_PROXY_TARGET` set to the server's address. A port that is
    already taken stops the run.
  - `web/vite.config.ts` forwards `/healthz` and `/api` to
    `SIDEREAL_DEV_PROXY_TARGET` when it is set, and to `http://localhost:5000`
    otherwise, so `mise run dev` and `mise run web` keep their behaviour.
  - A `trap` drops the database and removes the storage folder on every exit,
    pass or fail. If the database or folder cannot be created, the script
    exits non-zero before starting anything.
  - The restart step stops and starts only the server that this run started,
    against the same database and storage folder.
  - The CI `e2e` job no longer exports `DATABASE_URL` from `mise run db-url`,
    because the script makes its own database.

## Scrutiny Tier & Blast Radius

- **Tier:** high
- **Blast radius:** the public Source contract, both first-party packs, core
  persistence and filesystem boundaries, server composition and guarded API,
  startup configuration checks, the signed-in UI, the token generator, and
  the e2e runner and its CI job. Durable data and access checks are involved.

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

Migration 4 changes what existing specs and tests count. The
`postgres-foundation` delta moves first startup and concurrent startup from
three to four migration records and adds `assets` and `asset_versions` to the
application tables. The `security-events` delta stops naming the full table
list: "Nothing is stored" compares the table list before and after the acts.
Update `server/crates/core/src/database/startup/tests.rs` and the security
event test to match.

Alternative: a single table would erase the identity/version distinction.
Unique hashes on versions would prevent multiple versions sharing bytes.
Both alternatives contradict accepted decisions; no new data-model fork is
introduced here. This slice does not promise the later lookup of an existing
Asset on repeated bytes; storage sharing is independent of that lookup.

### 3. Stage a bounded stream and publish without replacing objects

Add an object-store service beside the existing StorageRoot pairing code,
constructed only from the root that passed startup pairing and probes.
The service opens the root directory once, with `O_DIRECTORY | O_NOFOLLOW`,
and keeps that descriptor for its lifetime. It then reads the marker through
that descriptor and refuses to start if the marker's UUID differs from the
UUID that pairing committed. Every later operation resolves relative to that
descriptor. A root that is swapped for another folder between pairing and
service construction therefore stops startup instead of receiving uploads.
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
exists-then-rename sequence is insufficient.

When an object already exists at the hash path, whether it was there before
or another request just published it, ingest opens it without following
links and checks that it is a regular file. It then reads the whole object,
computes its BLAKE3 hash, and compares hash and size with the candidate's
(D12). Only a match lets ingest sync the object and reuse it. A mismatch is
a storage failure: the response is 503, no Asset or version is committed,
and ingest leaves the existing object as it is, never overwriting, renaming
or deleting it. Reconciling a damaged object belongs to the later
verify-and-repair feature. Re-hashing costs one extra read of at most
100 MiB, only when bytes repeat.

Before committing, every upload syncs the whole directory chain: the leaf
hash directory, both prefix directories, `store/` and the root. It does this
whether this request created the directories or another request did. A
request cannot know whether another request that created a prefix directory
has synced it yet, so it never relies on that. Reject a filesystem boundary
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

The public web-file fallback serves all of `WEB_ROOT` with no session
(`server/src/lib.rs`, `with_web_root`). If managed storage sat inside
`WEB_ROOT`, the fallback would serve originals to anyone. So startup refuses
to run when `STORAGE_ROOT` and `WEB_ROOT` are the same folder, or one holds
the other, after both are resolved with symbolic links followed (D11). The
check runs in `config.rs` after both settings are read and before the
database is touched or HTTP is served. Its message names both `STORAGE_ROOT`
and `WEB_ROOT`. A symbolic link that someone later writes inside `WEB_ROOT`
is not checked; `docs/install.md` already says that only trusted people may
write to `WEB_ROOT`. A test at the production HTTP seam also requests a
stored object's path through the fallback with no session and gets no bytes.

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
Use the semantic color tokens below and existing spacing, type and radius
tokens. Do not add thumbnails, preview generation, image transforms, zoom,
sorting controls or a client router.

**New semantic tokens (D13).** The Library is the first component to need
these roles, and DESIGN.md says a semantic token is added when a component
first needs its role, in the same pull request. Each one is an alias of an
existing primitive, so no color value changes:

| Token | Points to | Role |
|---|---|---|
| `color-action-primary` | `{colors.violet-600}` | The filled Upload and Sign-in buttons |
| `color-text-on-action` | `{colors.white}` | The label on a primary action |
| `color-text-body` | `{colors.obsidian-100}` | Body text and filenames |
| `color-text-secondary` | `{colors.obsidian-400}` | Status and empty-library text |
| `color-border-hairline` | `{colors.obsidian-800}` | Field and card borders |
| `color-focus-ring` | `{colors.violet-500}` | Keyboard focus on Library controls |

Add them to `x-sidereal.semantic` in DESIGN.md first. Today
`web/scripts/tokens.ts` refuses every semantic token, and its error message
says to add reference lookup in the same pull request. So this change teaches
the generator to resolve a semantic token's reference to an existing
primitive and write it as `--sr-<token>: var(--sr-<primitive>)`. It still
refuses a semantic token whose value is a raw value or names no primitive,
and it still refuses component tokens. Then run `pnpm tokens` in `web/`.
The `design-tokens` delta records this. Penpot gets the tokens when an agent
next copies them, as DESIGN.md says.

DESIGN.md also says that once a semantic token exists, components use it
instead of its primitive. So `web/src/Access.css` and `web/src/styles/base.css`
switch from those primitives to these tokens where they serve these roles.
The computed colors stay the same, and the `web-shell` delta restates the
access section's token rule in semantic names.

No component tokens are added. Before building the new visual components,
use `visual-options` as DESIGN.md requires, record the selected component
treatment there, and verify it uses these tokens. That visual comparison can
adjust presentation within this plan. A choice that changes scope or needs
token values beyond this table returns to planning rather than silently
expanding the task.

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
serving HTTP and retains the existing storage UUID pairing. No new deployment
setting is required; the e2e and dev-proxy settings are for contributors only.
`tmp/` and `store/` appear on first upload, not at pairing. An installation
whose `STORAGE_ROOT` overlaps its `WEB_ROOT` stops at startup after the
upgrade and must move one of them; `docs/install.md` says so.

Migrations remain forward-only. An old binary must refuse the newer schema.
Prefer a forward fix. To restore a pre-change installation, stop the server
and restore its matched database and storage backup; reverting only the code
or deleting the new tables is not a supported rollback. Do not touch the
maintenance branch or its release process.
