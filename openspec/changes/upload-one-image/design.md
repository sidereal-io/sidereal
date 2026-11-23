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
  - It makes one run folder with `mktemp -d`. The run folder holds an empty
    `storage/` subfolder, which is the run's `STORAGE_ROOT`, plus the process
    files and one log for each server start (`server-1.log`, `server-2.log`,
    and so on). Nothing else is ever written into `storage/`: pairing refuses
    an unpaired root that holds other files (`core/src/database/pairing.rs`,
    rule 5).
  - It ignores an inherited `DATABASE_URL` and `STORAGE_ROOT`, so it can never
    point the server at a contributor's data.
  - It picks a server port and a Vite port from the checkout, the way
    `db-fixture.sh` picks the database port, so neither is 5000 or 5173 and
    two checkouts do not collide. It passes them to Playwright through
    `SIDEREAL_E2E_SERVER_PORT` and `SIDEREAL_E2E_WEB_PORT`.
  - Before it starts anything, the script refuses to run, with a non-zero
    exit, if anything already accepts connections on either chosen port.
  - Before it starts anything, the script runs `cargo build -p sidereal-server
    --manifest-path server/Cargo.toml`, so a fresh checkout has a binary. On
    CI the job's earlier build step makes this a no-op.
  - The script, not Playwright's `webServer`, starts
    `server/target/debug/sidereal-server` in the
    background with `PORT`, `DATABASE_URL` and `STORAGE_ROOT` set. The script
    is the only owner of server processes: every server it starts, the first
    and each replacement, is its own child. It runs Playwright as a background
    child and `wait`s on it, so it can handle signals while the tests run.
  - The browser test asks for a restart by sending `SIGUSR1` to the script.
    The script passes its own process id, from `$$`, to the test as
    `SIDEREAL_E2E_RUNNER_PID`. On `SIGUSR1` the script stops the current
    server and waits for it to exit. It starts the next one with the same
    settings and a new log, and waits for that log's listening line, as the
    next item says. Then it writes `ready-<n>` in the run folder, where `<n>`
    counts the restarts. The test waits for that file before it reloads.
    A trapped signal makes POSIX `wait` return early, so after a restart the
    script waits again on the same Playwright child. Only that child's exit
    ends the test run.
  - The script trusts the server only after positive evidence that its own
    child bound the port. The server initializes the database before it binds
    (`server/src/main.rs`), so another process on that port could answer
    `/healthz` first. The script therefore waits until the child it started
    prints `sidereal-server listening on` with the chosen port, and checks
    that the child is still running. Only then does it probe `/healthz`. A
    restart waits the same way. The wait is a shell function that a test can
    run on its own, with a stub child, so it can be tested with no timing
    race. It waits at most 300 seconds, as `web/playwright.config.ts` allows
    the server today.
  - `web/playwright.config.ts` reads the two port settings, starts only Vite,
    with `reuseExistingServer: false`, `--port <web port> --strictPort` and
    `SIDEREAL_DEV_PROXY_TARGET` set to the server's address. A port that is
    already taken stops the run.
  - The suite runs with one worker everywhere, locally and on CI, and with
    `fullyParallel` off, so no test runs during the upload journey's server
    restart (D17). The upload journey is marked serial and has no retries:
    a retry would find the instance already claimed. A failure fails the run
    once, with the first attempt's trace, so `trace` is `retain-on-failure`
    instead of `on-first-retry`.
  - `web/vite.config.ts` forwards `/healthz` and `/api` to
    `SIDEREAL_DEV_PROXY_TARGET` when it is set, and to `http://localhost:5000`
    otherwise, so `mise run dev` and `mise run web` keep their behaviour.
  - A `trap` cleans up on every exit, pass or fail, in this order. It stops
    Playwright's process tree, Vite and Chromium included, and reaps the
    Playwright child, as `scripts/dev.sh` cleans up its descendants. It stops
    whichever server is running at that moment, including one that a restart
    has just started, and waits for it to exit. It removes the run folder. It drops the database with the policy of
    `core/src/database/test_support.rs`: up to three rounds, each trying
    `DROP DATABASE ... WITH (FORCE)` and falling back to a plain
    `DROP DATABASE` when PostgreSQL refuses the forced drop with `42501`.
    Each step runs even if an earlier one failed. The run's exit status is
    the test result, except when the tests passed and cleanup failed: then
    the run exits non-zero and names what it left behind. `db-fixture.sh
    orphan-clean` still removes a database left behind.
  - If the database or folder cannot be created, the script exits non-zero
    before starting anything.
  - A restart stops and starts only the server that this run started,
    against the same database and storage folder.
  - The CI `e2e` job no longer exports `DATABASE_URL` from `mise run db-url`,
    because the script makes its own database. The job still builds the
    server first, so compiling stays out of the runner's 300-second wait for
    the listening line. Verify commands that read the workflow use Mike
    Farah's `yq` v4, run as `mise exec yq -- yq`; this change does not pin it.

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
list: "Nothing is stored" compares the table list before and after the acts,
and checks that only `users` and `sessions` change rows. It gets its own test
that performs all five acts. Update
`server/crates/core/src/database/startup/tests.rs` to match.

The deployment verifier counts migrations too: `scripts/instance-verify.sh`
asserts three successful migrations. It moves to four, with its message, and
the `compose-instance` delta moves that requirement's count to four as well
(the main spec still says two, from before the access migration).

Alternative: a single table would erase the identity/version distinction.
Unique hashes on versions would prevent multiple versions sharing bytes.
Both alternatives contradict accepted decisions; no new data-model fork is
introduced here. This slice does not promise the later lookup of an existing
Asset on repeated bytes; storage sharing is independent of that lookup.

### 3. Stage a bounded stream and publish without replacing objects

Startup resolves `STORAGE_ROOT` once, with symbolic links followed, whether
or not `WEB_ROOT` is set. Pairing, the web-root overlap check and the object
store all use that resolved path. A `STORAGE_ROOT` that is a symbolic link to
a valid folder therefore keeps working, as `StorageRoot::new`
(`core/src/storage.rs`) allows today.

Add an object-store service beside the existing StorageRoot pairing code,
constructed only from the resolved root that passed startup pairing and
probes. The service opens that root directory once, with `O_DIRECTORY`, and
keeps that descriptor for its lifetime. The root itself is already resolved,
so it is opened without `O_NOFOLLOW`; every component below it still refuses
symbolic links. It then reads the marker through
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
After the field completes, the handler keeps reading parts until the closing
delimiter, and refuses any extra field before executing or committing an
upload. The closing delimiter ends validation (D19). Bytes after it are a
multipart epilogue, which RFC 2046 says to ignore; the handler does not read
them. axum 0.8's multipart reader stops at the closing delimiter in the same
way. The file is complete at that point, so a later error in the rest of the
body does not change the outcome, just as a lost response after commit does
not. A body that ends or fails before the closing delimiter is malformed: it
gets 400 and commits nothing. Set the route's transport limit above 100 MiB to allow bounded
multipart overhead (1 MiB); enforce the exact 100 MiB file cap separately.
The transport limit counts the whole body, epilogue included, so a request
whose epilogue pushes it past 101 MiB gets 413. The `image-upload` spec
states this bound. Browsers send no epilogue, so no real upload meets it.
An overlong or malformed multipart envelope gets a clear client error. Leave
the body limits of access endpoints unchanged.

After staging, core reads the candidate to calculate BLAKE3 and its byte size,
and the selected Source reads its signature through the context. Flush and
sync the accepted file before publication. Use an atomic rename that refuses
to replace an existing entry, with a platform-supported no-replace operation;
an unsupported primitive fails closed. A plain overwriting rename or
exists-then-rename sequence is insufficient.

When an object already exists at the hash path, whether it was there before
or another request just published it, ingest opens it with
`O_NOFOLLOW | O_NONBLOCK`, as `open_marker` in `core/src/storage.rs` does,
and checks that the opened file is a regular file. A FIFO or other special
file at the hash path is refused at once instead of blocking the open. It first compares the open
object's size with the candidate's, and refuses on a mismatch without
hashing. Only a same-size object is read and hashed with BLAKE3, and the read
stops as soon as the object has given more bytes than the candidate holds, in
case the file grows meanwhile (D12). Only a matching hash and size let ingest
sync the object and reuse it. A mismatch is
a storage failure: the response is 503, no Asset or version is committed,
and ingest leaves the existing object as it is, never overwriting, renaming
or deleting it. Reconciling a damaged object belongs to the later
verify-and-repair feature. Re-hashing costs one extra read of at most the
candidate's size, never more than 100 MiB, only when bytes repeat.

Before committing, every upload syncs the whole directory chain, in this
order: the second prefix directory (`store/{h[0:2]}/{h[2:4]}`, which holds
the object file), the first prefix directory (`store/{h[0:2]}`), `store/`
and the root. The full hash names the object file, not a directory. It does this
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
check uses the resolved `STORAGE_ROOT` from decision 3. It runs in
`config.rs` after both settings are read and before the database is touched
or HTTP is served. Its message names both `STORAGE_ROOT` and `WEB_ROOT`.

Startup also resolves `WEB_ROOT` once, with symbolic links followed (D18).
That one resolved folder is used for the overlap check, the `index.html`
check, `ServeFile` and `ServeDir`. Today `config.rs` keeps the configured
path, and `ServeDir` follows it again on every request (`main.rs`,
`with_web_root` in `lib.rs`). If `WEB_ROOT` were itself a link, someone who
repointed it after startup at a folder holding the store would expose
originals. With the resolved folder, a repointed `WEB_ROOT` link takes
effect only after a restart, which runs the check again.

Two cases are outside the check (D11, D15): a symbolic link that someone
writes inside `WEB_ROOT`, and a bind mount that exposes the storage folder
inside `WEB_ROOT`. Resolving paths cannot see a bind mount, and making one
needs the same trust as writing into `WEB_ROOT`. `docs/install.md` already
says that only trusted people may write to `WEB_ROOT`; it gains a sentence
that no mount or link may expose `STORAGE_ROOT` inside `WEB_ROOT`. A test at
the production HTTP seam also requests a stored object's path through the
fallback with no session and gets no bytes.

The API uses these shapes:

- `POST /api/assets`: multipart field `file`; 201 `{ "asset": <entry> }`.
- `GET /api/assets`: 200 `{ "assets": [<entry>, ...] }`, with entries ordered
  by ingestion time and id for stable rendering in this slice.
- `<entry>`: `id`, `kind`, `name`, `current_version_id`, `original_url`.
- `GET /api/assets/{id}/original`: stream the file resolved from the persisted
  current version's hash, with the pack-recognized Content-Type, nosniff and
  no-store headers on every 200. Invalid or unknown ids get 404. Guard-made
  401s carry no file bytes and keep their existing headers.

A 503 is for a failure found before the response starts: the object is
missing, cannot be opened, or is not a regular file, or storage is not
configured. Retrieval opens the object with `O_NOFOLLOW | O_NONBLOCK` and
checks the opened file, as ingest does, so a FIFO returns 503 promptly. A read that fails after the headers are sent cannot change the
200. The server then aborts the response before its end, so the client sees
a failed transfer and never a complete body, and it logs the failure. The
browser shows the display error.

GET does not re-hash the object it streams (D16). It returns the bytes
stored at the version's hash path. Re-hashing up to 100 MiB on every view
would cost a full read per view, and a mismatch found mid-stream cannot undo
a 200 that has already been sent. Detecting a damaged or replaced object
belongs to the later verify-and-repair feature.

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
health display and Access's credentials and socket transitions intact. Use
one type=button Upload action that opens a hidden, labelled single-file input
accepting PNG/JPEG. Choosing a file sends it at once, with no form and no
separate choose step. Mounting loads the list; success reloads it; list
errors have Retry. A refused upload names the file in its message and lets
Nova choose another file at once, the same one included.
Client size/type hints do not replace server validation. On 401, notify Access
to return to sign-in. Unmount aborts outstanding fetches and ignores late
responses. Image load errors retain the name and show the fixed display message.

Use the Showcase layout that DESIGN.md's Library section records. App
owns one top bar: the `Sidereal` wordmark, the screen's name, the health
line, and a place for the screen's actions. The Library puts its name and
its actions there: one primary Upload button, then the username and a
quiet Sign out from Access. The bar and the health line stay mounted when
the screen changes. Below the bar there is only the grid: uncropped
originals at their natural ratio in justified rows that aim for `size-300`
tall, square corners, and the filename under each in the mono face. The
list API returns no width or height, so each frame keeps a 3:2 shape until
its image loads and gives its own ratio. Every other state is one quiet
line of text under the bar, and a list error adds a quiet Retry button.
Display status colors only in badges. Use the semantic color tokens below
and existing spacing, type and radius tokens, plus one new primitive,
`size-300`, for the row height. Do not add thumbnails, preview
generation, image transforms, zoom, sorting controls or a client router.

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

The generator's escape-safety rule refuses any value that holds `{` or `}`,
so as written it refuses every reference. The `design-tokens` delta narrows
that value rule to primitives. A semantic value must match exactly
`{<group>.<key>}`, where the key holds only lowercase letters, digits and
hyphens, and must name an existing primitive. The generator never writes the
raw text of a semantic value, only the resolved `var(--<prefix>-<key>)`. So a
semantic value with extra text, such as `"{colors.violet-600}; } body {"`,
is refused.
The `design-tokens` delta records this. The Penpot export
(`web/scripts/penpot-tokens.ts`) carries primitives only, so it does not
carry these six semantic tokens yet. Follow-up issue #533 teaches the exporter
aliases; that goes beyond this story. `web/README.md` changes to show the
accepted `{<group>.<key>}` form and the cases still refused.

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
expanding the task. The user rejected the Penpot options and chose the
Showcase layout from a set of HTML drafts instead. It needs one value
beyond this table, the `size-300` row height, which the user asked to add
to DESIGN.md; the tasks' Deviations Ledger records both.

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
upgrade and must move one of them; `docs/install.md` says so. A deployment
that repoints a `WEB_ROOT` link to switch web builds must restart the server
for the new build to be served; `docs/install.md` says that too.

Migrations remain forward-only. An old binary must refuse the newer schema.
Prefer a forward fix. To restore a pre-change installation, stop the server
and restore its matched database and storage backup; reverting only the code
or deleting the new tables is not a supported rollback. Do not touch the
maintenance branch or its release process.
