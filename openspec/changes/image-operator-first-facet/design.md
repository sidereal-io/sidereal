<!-- Plain language: Help the next implementer find the approach, understand the choices and act on the plan. Explain necessary technical terms. Put each risk beside its mitigation and each open question beside the decision it affects. -->

## Context

This change stacks on `upload-one-image` (#521). That branch already has:

- **`plugin-abi`** (`server/crates/plugin-abi/src/lib.rs`): an executable
  `Source` that runs through `AssetContext` (bounded reads, one proposal, log,
  cancellation), and `Registry::execute_source`, which returns the proposal
  with host-made provenance. `Operator` is still an id-only stub. ABI
  version 1.
- **`packs/image`**: the manual-upload Source. `packs/astro`: stubs,
  including `PlateSolveOperator`.
- **`server/scripts/check-arch.sh`**: lints every pack under `crates/packs/`
  and refuses a pack that depends on core, the server or another pack. The
  story's arch-lint scope is already met.
- **core**: `ingest()` runs the Source, publishes the object, then commits the
  Asset and its first version (`assets.rs`). `ObjectStore::open_object(hash)`
  opens a stored object safely. Migrations 1–4, with a JSON schema snapshot
  per migration in `core/schema/` that `startup` checks at every start.
- **server** (`server/crates/server/src/assets.rs`): `POST /api/assets`,
  `GET /api/assets`, `GET /api/assets/{id}/original`, all inside the session
  guard. `Library` holds the object store and the production Registry.
  `with_web_root` serves `/` as `index.html` and other files through
  `ServeDir`, with no `index.html` fallback.
- **web**: no router. `Access.tsx` renders `Library` in its signed-in branch
  and passes the username and Sign out button into the top bar through
  `Bar.tsx`. Vite writes the built scripts and styles under `/assets/`, the
  same prefix as the new page's address.

Constraints: packs depend on `plugin-abi` only; core knows no astronomy and
no image facts; core is the only writer of facet values (ADR-008); the
`plugin-abi` contract is public but unfrozen. Sibling stories #522 and #523
also build on `upload-one-image` and touch the image pack, so this change
renames nothing that already exists in the contract.

## Goals / Non-Goals

**Goals:**

- One Operator path from a pack to stored, checked facet values, shaped so
  later Operators (#525, #526, plate solving) only add code, not contract
  changes for the same concern.
- All checks that decide what is stored live in core, not in the pack and not
  in the contract crate.
- The upload journey stays as it is for the client, apart from the extra fact.

**Non-Goals:**

- No job queue, background worker, retry, or reconciler. The one run happens
  inline in the upload request (D5 in the brainstorm).
- No selection rule for an Operator (`accepts` Selector) and no processing
  goals (#527). The server names the one Operator that upload runs.
- No current-value rule when two producers write the same facet. Only one
  producer exists.
- No facet value history, no re-run, no backfill (D2).
- No value types other than a whole number of zero or more.
- No router library and no general single-page-app fallback on the server.

## Test Seams

- **Primary test seam:** the server's in-process route tests
  (`server/crates/server/tests/assets.rs`), against a fresh PostgreSQL
  database and paired storage per test. They drive upload, the new
  `GET /api/assets/{id}`, and the failure cases, and read `operation_runs`
  and `facet_values` through SQL, as the existing tests read `assets`. They
  own the outcomes only the server can show: an Operator panic leaves the
  server answering the next request, and a client that goes away during the
  commit still gets its run (decision 5).
- **Database fixture:** every test against PostgreSQL (core's database
  tests and the server's route tests) needs the fixture that CONTRIBUTING.md
  names: `mise run db-up`, then
  `export TEST_DATABASE_URL=$(mise run db-test-url)`. Without it,
  `TestDatabase::new` stops the test before any assertion.
- **Existing seams reused:**
  - `plugin-abi`'s unit tests (`mod source_execution` pattern) for facet
    declarations, the namespace rule and Operator execution with in-memory
    bytes.
  - core's database tests (`core/src/ingest/tests.rs` pattern, with
    `test_support::TestDatabase`) for the executor's checks, run records and
    provenance, using test Operators registered through the public contract.
  - core's schema-snapshot test (`startup/tests.rs`) and upgrade test for
    migration 5.
  - `server/tests/web_root.rs` for the `/assets/<id>` answer to `GET` and
    `HEAD`, the files `index.html` names, and a file at a reserved address.
    The browser journey runs through Vite, so this is the seam for the built
    shell.
  - Vitest with jsdom (`web/src/*.test.tsx`) for the asset page,
    navigation and the upload that goes on while the asset page shows (a
    test holds the upload's `fetch` pending, opens an image, goes back, and
    checks the status line, the disabled button and that no second request
    went out), and a Vitest test that reads `AssetPage.css` for the
    "uses tokens" scenario.
  - The browser journey (`web/e2e/upload.spec.ts`, run by `mise run e2e`) for
    upload → open the page → see the file size → reload.

## Scrutiny Tier & Blast Radius

- **Tier:** high. The change adds a public contract (Operator and facet
  declarations), a migration with three tables, and a trust boundary: which
  plugin may write which fact.
- **Blast radius:** `plugin-abi` (public API, ABI version 2), both packs,
  core's database schema, a new `operations` module and the `User` type
  (which gains the user's id), the upload route and
  the web-file fallback in the server, and the web app's signed-in view,
  where the upload state moves from `Library` to the signed-in view.
  Existing Assets and versions are read but never changed.

## Decisions

### 1. One `AssetContext` for Sources and Operators

The glossary defines one *Asset context* through which a plugin reads its
input's bytes and proposes facets and assets. The contract keeps that one
type. It gains `propose_facet(name, FacetValue)`. Its input reader keeps the
names `candidate_size` and `read_candidate`; for an Operator run, the
candidate is the Asset version it reads, and the doc comments say so.

Each run kind allows its own proposals. A Source run refuses
`propose_facet` with a new `ContextError::NotAllowed`, until a story lets
Sources propose detected facets. An Operator run refuses `propose_asset` the
same way, until derived assets arrive (#526). A refused call also marks the
run broken, as a second asset proposal does today, so the run fails.

- *Alternative:* a separate `OperatorContext`. It prevents a wrong call at
  compile time, but splits one glossary concept into two types, and the
  next stories (Sources with detected facets, Operators with derived assets)
  would merge them again.
- *Alternative:* rename `read_candidate` to `read_input`. Clearer, but #522
  and #523 change the same Source code in parallel. A rename can wait for
  the Operator API freeze.

### 2. The Operator trait and facet declarations

```rust
pub trait Operator: Send + Sync {
    fn id(&self) -> &str;
    fn version(&self) -> &str;
    /// The facets this Operator writes: its grants.
    fn writes(&self) -> &[&str];
    fn run(&self, context: &mut AssetContext<'_>) -> Result<(), OperatorError>;
}

pub struct FacetSchema {
    pub name: String,        // "image.file-size"
    pub label: String,       // "File size"
    pub value_type: FacetType,   // FacetType::Count: whole number >= 0
    pub unit: String,        // "bytes"
    pub version: u32,        // 1
    pub scope: FacetScope,   // Asset | AssetVersion
}

pub enum FacetValue { Integer(i64) }

/// The largest count: the largest whole number a JavaScript client reads
/// exactly (`Number.MAX_SAFE_INTEGER`).
pub const MAX_COUNT: i64 = 9_007_199_254_740_991;
```

`FacetValue::Integer` takes any `i64`, so an Operator can propose -1, or a
number above `MAX_COUNT`, and core can refuse it, as the spec's scenarios
need. A `Count` fits when it is from 0 to `MAX_COUNT`. The API sends the
value as a JSON number, and the page reads it exactly, so core, the API and
the page accept the same range (decision 8).

`OperatorError` mirrors `SourceError` without `Refused`: `Unsupported` and
`Failed(String)`.

**Core cleans every reason before it records it.** A reason can hold text
from the Operator: a `Failed` reason, or a proposed facet name inside a
refusal. PostgreSQL `TEXT` cannot hold U+0000, so an update with one fails on
a healthy database and leaves the run `running`. Core therefore removes every
U+0000 from the reason first. A reason that is then empty, or only spaces,
is recorded as `the operator gave no reason`, so the stored reason is never
empty and the `btrim(reason) <> ''` check always holds. One function in
`operations` does this, and every update that records `failed` calls it.

A pack declares a schema with `Registry::declare_facet(schema)`. The host
registers each pack through a new `Registry::register_pack(&dyn Pack) ->
Result<(), RegistryError>`. It creates an empty staging Registry, notes the
pack's id, and calls `Pack::register` on the staging Registry. It then
checks what the pack declared and registered there against the live
Registry and against itself:

- a facet name already declared, or one that does not start with
  `<pack id>.`, is refused with an error that names the facet;
- an Operator id that another registered Operator already has is refused
  with an error that names the id;
- a declaration that core could not store is refused with an error that
  names the facet or the Operator. These checks match migration 5's
  columns (decision 4): a facet name matches
  `^[a-z0-9-]+(\.[a-z0-9-]+)+$`; a schema version is from 1 to `i32::MAX`
  (2,147,483,647), because `facet_schemas.version` is `INTEGER`; a label is
  not empty; an Operator id and version are not empty; and no label, unit,
  Operator id or Operator version holds U+0000. So a run never fails on a
  healthy database because of what its pack declared.

Only when every check passes does it move the staged Sources, Operator
entries, Sinks and schemas into the live Registry. A refused pack therefore
adds nothing, and every pack registered before it stays as it was. A
`Pack::register` that panics also leaves the live Registry unchanged.

**Every host registers packs through `register_pack`.** `Pack::register`
stays public, because a pack implements it, but its doc comment says that
hosts call `register_pack` and never `Pack::register` directly. This change
moves every existing direct call to `register_pack`:
`server/crates/server/src/assets.rs` (`Library::new`),
`packs/image/tests/recognition.rs`, `packs/astro/tests/registration.rs` and
`core/src/ingest/tests.rs`. A test that needs a refused pack calls
`register_pack` and checks its error. The `register_operator` and
`declare_facet` methods stay too, because `Pack::register` uses them, but
only the staging Registry that `register_pack` hands a pack is meant to
receive them.

**The grant is fixed at registration.** When an Operator is registered, the
Registry calls its `id()`, `version()` and `writes()` once and keeps the
answers in a host-owned entry. The Registry never calls `writes()` again.
`Registry::execute_operator` takes the producer and the grants from that
entry, so an Operator that answers `writes()` differently later (through
interior mutability) cannot widen its grants or change its identity. This is
the registration-time approval that D4 describes, and the place where
plugin installation (#361) will later add its own approval step (ADR-007).

`Pack::register` keeps its signature, so packs do not handle errors. The
server builds its Registry through `register_pack` for both packs. A
conflict among compiled-in packs is a build defect: `Library::new` panics
with the error's message, and a server unit test builds the production
Registry so the defect fails `mise run check`, not a start.

`ABI_VERSION` becomes 2. The astro `PlateSolveOperator` stub gains a version,
an empty `writes()`, and a `run` that returns `OperatorError::Unsupported`.

`Registry::execute_operator(id, candidate, log, cancelled)` runs the
Operator and returns its proposals with host-made provenance
(`producer_id`, `producer_version`) and the grants, all from the
registration entry, or an `ExecutionError`. It does not check grants or
schemas: core does (decision 3).

A schema's `scope` says what the value describes. Core stores an
`AssetVersion` value against the version the run read, and an `Asset` value
against the Asset that owns that version (decision 3). The file-size facet
is an `AssetVersion` facet.

### 3. Core's executor checks everything before it stores anything

A new core module, `operations`, holds

```rust
pub async fn run_operator(
    database: &Database,
    objects: &ObjectStore,
    registry: Arc<Registry>,
    run: RunRequest,            // operator_id, asset_version_id, actor: i64 (users.id)
    cancelled: Arc<AtomicBool>,
) -> Result<RunOutcome, OperationError>;

pub enum RunOutcome { Succeeded, Failed(String) }
pub enum OperationError { Unrecorded(DatabaseError) }
```

`RunOutcome` is what the run's row says. `OperationError::Unrecorded` means
core could not record the run's start, or could not record or read back how
it ended; the outcome is then unknown. In order:

1. Read the version's row: its Asset id, content hash and recorded
   `byte_size`. Insert an `operation_runs` row with status `running`, the
   Operator's id and version from the registration entry, the version id,
   the actor and `started_at = now()`, in its own transaction, so a failed
   run is still recorded.
2. On a blocking worker, open the version's object with
   `ObjectStore::open_object(content_hash)`. If the opened file's length is
   not the version's `byte_size`, the run fails with the reason
   `the stored object does not match its version`, and the Operator does
   not run. Otherwise expose the file as a `Candidate` through positioned
   reads, with `size()` equal to `byte_size`, and run
   `Registry::execute_operator`. Log lines go to standard error as
   `operator <id>: <line>`, as Source lines do.
3. Check each proposal: a registered schema exists for the name (unknown);
   the name is in the registration entry's grants (grant); the value fits
   the schema's type (a `Count` is from 0 to `MAX_COUNT`). Check
   cancellation once more.
4. In one transaction: write each used schema's row (decision 4), insert
   every value into `facet_values` against the subject its scope names, and
   set the run to `succeeded` with `ended_at`. On any failure in steps 2–4,
   set the run to `failed` with its reason and `ended_at`, and store no
   value.

A panic in the Operator ends the blocking task with a join error; core
records `failed` with the reason `the operator stopped`. An `Unsupported` or
`Failed(reason)` result is recorded with that reason. A refused proposal's
reason names the facet, for example `facet test.width is not granted to
test.op`. Every reason is cleaned first (decision 2).

**A finished run never changes.** Every update that ends a run, to
`succeeded` or to `failed`, has the condition
`WHERE id = $1 AND status = 'running'`. So a run that is `failed` never has
stored values, and a run that is `succeeded` is never marked `failed`.

**When the final commit's answer is lost.** If the commit in step 4 returns
an error, core cannot tell whether PostgreSQL committed it. Core then sends
the conditional failure update on a fresh connection, with the reason
`the values could not be stored`:

- one row changed: the commit did not land; the outcome is `Failed`;
- no row changed: the commit landed; core reads the status back and
  returns it (`Succeeded`);
- the update or the read fails: core returns `Unrecorded`, and the row may
  stay `running`, as a server stop leaves it (see Risks).

Core takes schemas and grants from the Registry at run time, so it never
trusts what a pack wrote into the database earlier. The `facet_schemas`
rows keep each schema version as values were written under it (decision 4).

### 4. The tables: migration 5

```sql
CREATE TABLE public.operation_runs (
    id UUID PRIMARY KEY,
    operator_id TEXT NOT NULL CHECK (operator_id <> ''),
    operator_version TEXT NOT NULL CHECK (operator_version <> ''),
    asset_version_id UUID NOT NULL REFERENCES public.asset_versions (id),
    actor_user_id BIGINT NOT NULL REFERENCES public.users (id),
    status TEXT NOT NULL CHECK (status IN ('running', 'succeeded', 'failed')),
    reason TEXT CHECK (btrim(reason) <> ''),
    started_at TIMESTAMPTZ NOT NULL,
    ended_at TIMESTAMPTZ,
    CHECK ((status = 'failed') = (reason IS NOT NULL)),
    CHECK ((status = 'running') = (ended_at IS NULL)),
    CHECK (ended_at >= started_at)
);

CREATE TABLE public.facet_schemas (
    name TEXT NOT NULL CHECK (name ~ '^[a-z0-9-]+(\.[a-z0-9-]+)+$'),
    version INTEGER NOT NULL CHECK (version >= 1),
    owner_pack TEXT NOT NULL CHECK (owner_pack <> ''),
    label TEXT NOT NULL CHECK (label <> ''),
    value_type TEXT NOT NULL CHECK (value_type <> ''),
    unit TEXT NOT NULL,
    scope TEXT NOT NULL CHECK (scope IN ('asset', 'asset_version')),
    PRIMARY KEY (name, version)
);

CREATE TABLE public.facet_values (
    id UUID PRIMARY KEY,
    facet TEXT NOT NULL,
    schema_version INTEGER NOT NULL,
    asset_id UUID REFERENCES public.assets (id),
    asset_version_id UUID REFERENCES public.asset_versions (id),
    value JSONB NOT NULL CHECK (jsonb_typeof(value) <> 'null'),
    producer_id TEXT NOT NULL CHECK (producer_id <> ''),
    producer_version TEXT NOT NULL CHECK (producer_version <> ''),
    run_id UUID NOT NULL REFERENCES public.operation_runs (id),
    written_at TIMESTAMPTZ NOT NULL,
    FOREIGN KEY (facet, schema_version) REFERENCES public.facet_schemas (name, version),
    CHECK ((asset_id IS NULL) <> (asset_version_id IS NULL))
);
CREATE INDEX facet_values_asset ON public.facet_values (asset_id);
CREATE INDEX facet_values_asset_version ON public.facet_values (asset_version_id);
CREATE INDEX facet_values_value ON public.facet_values USING GIN (value jsonb_path_ops);
```

- **One row per value, with the value in a `JSONB` column** and a GIN index
  on it, as the user chose and
  [ADR-017](../../../docs/decisions/ADR-017-facet-value-rows.md) records. The
  row keeps each value's provenance in typed columns, with foreign keys to its
  schema version, its Asset or Asset version, and its run. A file size is
  stored as a bare JSON number, such as `1234567`. A new value type needs no
  migration: `value_type` is free text that core checks, as it checks values.
- **The actor is on the run.** ADR-008's provenance names an actor: who
  caused the write. Every run in this change starts from an upload, so its
  actor is the signed-in administrator, stored as `actor_user_id`. Each value
  reaches its actor through `run_id`, as it reaches the run's times. User
  rows are never deleted, so the reference stays valid. `users.id` is
  `BIGINT GENERATED ALWAYS AS IDENTITY` (migration 3), so the column is
  `BIGINT` and core's `User` type gains `id: i64`. Every place that builds a
  `User` fills it in: the claim insert in `access/claim.rs` (with
  `RETURNING id`), sign-in in `access/session.rs` (from the row it checks the
  password against), the session lookup in `access/session.rs` (which adds
  `u.id` to its `SELECT`), and the test values in both files. The session
  guard then hands the id to the upload handler.
  A later story that starts runs without a person (processing goals, #527)
  decides how to record that actor.
- **Core checks the value's type, not PostgreSQL.** A row check cannot read
  the schema's `value_type` in another table, so the database only refuses
  JSON `null`. Core checks every value against the Registry's schema before it
  stores it (decision 3), and only core writes the table (ADR-008).
- **Writing and reading the column.** The workspace's `sqlx` gains its `json`
  feature, so core binds and reads `serde_json::Value`. Core turns
  `FacetValue::Integer(n)` into a JSON number, and reads a `count` value back
  with `as_i64()`; a stored value that is not a whole number from 0 to
  `MAX_COUNT` is a database error, not a fact.
- **The GIN index uses `jsonb_path_ops`,** which serves containment (`@>`)
  and JSON path matches. Range comparisons on one facet, which later
  Selectors need, will add an expression index for that facet in the story
  that needs it.
- *Alternative:* a typed column per value type (`count_value BIGINT`).
  PostgreSQL could check and range-index the value, but every new value type
  needs a core migration. *Alternative:* ADR-004's literal rule, a `facets`
  column on `assets` and `asset_versions`. It changes version rows that
  `asset-library` keeps immutable, and the database cannot check a value's
  run reference. ADR-017 gives the full reasons.
- **One `facet_schemas` row per schema version, never changed.** A row is
  written when a value first needs it, inside the run's transaction, with
  `INSERT … ON CONFLICT (name, version) DO NOTHING`. Core then reads the row
  back and compares it with the running build's declaration. If the label,
  unit, value type, scope or owner differ, the run fails with the reason
  `facet <name> version <n> changed without a new version`, and stores
  nothing. So a value is always read with the label and unit of the schema
  version it was written under, and a build that changes a facet's meaning
  must raise its version (ADR-008). Startup writes nothing new, so the
  readiness, restore and "nothing is stored" checks keep their behavior, and
  the upgrade scenario's tables are empty.
- **No unique key on (facet, version).** ADR-008 keeps every observation; a
  rule for which value is current is not part of this change.
- Mint all ids in core with `Uuid::new_v4()`, as `assets.rs` does.
- Regenerate `core/schema/5.json` with `mise run db-schema` and add it to
  `baselines()`.

### 5. Upload runs the file-size Operator in the task that commits

Once the upload is staged, the handler calls `tokio::spawn` with one task
that runs `ingest()` and then, when ingest returns the saved record,
`run_operator(…, RunRequest { FILE_SIZE, version id, actor: user.id }, …)`.
The handler awaits the task's join handle. A spawned task keeps running when
its handle is dropped. So once the task has started, a client that goes
away cannot stop the commit or the run: there is no point after the commit
where the run has not yet been started. This closes the gap where the
request future is dropped while the commit is pending.

The two cancel flags keep their jobs. `CancelOnDrop` sets ingest's flag when
the request is dropped. Ingest checks it during the Source run and once
before publication, as it does today, so a client that goes away before
then still leaves nothing behind. The run gets its own flag, which the
request never sets.

The task writes the log line itself, because the handler may be gone.
Whatever the run's result, the upload answers 201 with the existing body:

- `RunOutcome::Failed(reason)` writes
  `file-size run failed for asset <id>: <reason>`;
- `OperationError::Unrecorded` writes
  `file-size run not recorded for asset <id>: <error>`. This tells the
  operator that the outcome is unknown, unlike a failed or refused run,
  which is recorded and stored no value. When the run's first insert was
  refused, nothing of the run exists and the Asset has no facets. When the
  final commit landed but its answer and the read-back were both lost, the
  values and the `succeeded` row exist, and reading the Asset returns them.
  Tests assert `"facets":[]` only for a run that definitely stored nothing.

A join error of the whole task (a panic in core, not in the Operator) is a
server fault: the handler answers 500 as other upload faults do and logs
`upload failed: the upload task stopped`.

The image pack exports `FILE_SIZE` (the Operator id `image.measure-file-size`)
and `FILE_SIZE_FACET` (`image.file-size`). The Operator proposes
`context.candidate_size()` as the value. It reads no bytes: the size is the
fact, and core has already checked that the stored object's length is the
version's recorded size (decision 3, step 2).

For tests, `Library` gains two `#[doc(hidden)]` builders, like
`with_original_reader`: `with_registry(Registry)` and
`with_upload_operators(Vec<String>)`. A test registers the image pack plus a
test pack whose Operator fails, panics, blocks on a channel, or proposes a
refused value, and points upload at it.

The commit gap has its own route test. The test holds
`LOCK TABLE public.assets IN ACCESS EXCLUSIVE MODE` in an open transaction,
starts an upload, and waits until `pg_locks` shows the upload's insert
waiting on that lock: ingest has then passed its last cancellation check.
The test drops the request, releases the lock, and checks that the Asset,
a `succeeded` file-size run and the file-size value all exist.

### 6. `GET /api/assets/{id}` returns the Asset with its facets

The route sits in `assets::routes`, inside the session guard. It parses the
id like the original route (404 for a malformed id), reads the Asset, then
calls a new `Database::facets(asset_id, current_version_id)`. That query
joins each value in `facet_values` to its own schema version's row in
`facet_schemas`, on `(facet, schema_version)`, for values whose `asset_id`
is the Asset or whose `asset_version_id` is its current version. It orders
them by facet name, then `written_at`, then id. A database error is 503
with the library's fixed message.

The body wraps the entry in `asset`, as upload's 201 body does:

```json
{"asset": {"id": "…", "kind": "image", "name": "nebula.png",
           "current_version_id": "…", "original_url": "/api/assets/…/original",
           "facets": [{"name": "image.file-size", "label": "File size",
                       "value": 1234567, "unit": "bytes"}]}}
```

The route is a read. The origin check lets `GET` and `HEAD` pass
(`access/guard.rs`, `origin`), as it does for the list and the original, so
its tests assert 401 without a session and no origin refusal. The
forbidden-origin regression stays with upload's tests.

### 7. The server answers `/assets/<id>` with `index.html`

`with_web_root` wraps its file fallback in one small service. When the path
matches `^/assets/[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$`,
it answers `GET` and `HEAD` with the same `ServeFile` for `index.html` that
`/` uses. Every other path goes to `ServeDir` unchanged, so Vite's
`/assets/index-….js` files, a tester's `assets/index.html`, and every 404
keep working. The match is on the exact text: an uppercase id or a trailing
slash is not an asset page address. Vite's output folder stays where it is.

`ServeDir` percent-decodes the path before it looks for a file, so
`/assets/%30b6f3c2e-…` would reach the file at the reserved path
`assets/0b6f3c2e-…`. The wrapper therefore also decodes the path, with the
same `percent_decode(…).decode_utf8()` that `ServeDir` uses. When the
decoded path is an asset page address but the path as written is not, it
answers 404 itself, and `ServeDir` never sees the request. A `web_root` test
requests the encoded spelling of the reserved file's address.

An asset page address is reserved. A file or folder in `WEB_ROOT` whose path
is an asset page address is never served: the address answers with
`index.html`. Vite names its files with a hash and an extension, so no built
file has such a path. A `web_root` test puts a file at one such path and
checks that the address still answers with `index.html`.

- *Alternative:* an axum route `/assets/{id}`. It would capture every
  one-segment path under `/assets/`, including Vite's own files, so it would
  have to forward non-matching paths to `ServeDir` itself. The wrapper does
  the same with less code and keeps "routes win over files".
- *Alternative:* move Vite's output to `/static/`. It removes the shared
  prefix but still needs the same matcher, and changes the built shell's
  layout for no gain.

The Vite dev server already answers unknown HTML requests with `index.html`,
so `mise run dev` and `mise run e2e` need no change.

### 8. Navigation in the web app without a router library

A new `web/src/navigation.ts` holds a `usePath()` hook, which reads
`window.location.pathname` and listens for `popstate`, and a
`navigate(path)` function, which calls `history.pushState` and notifies the
hook. A `Link` component renders a real `<a href>` and calls `navigate` on a
plain left click (no modifier key), so opening in a new tab still works.

`Access.tsx`'s signed-in branch renders a new `SignedIn` component, which
picks the screen from the path: `/assets/<id>` (the same pattern as
decision 7) shows `AssetPage`; any other path shows `Library`. Both get
`onSessionLost` and the account items (username and Sign out) as children
for the top bar, so the bar looks the same on both screens. When the session
ends, Access shows the sign-in form and keeps the path, so signing in again
shows the same screen.

**The upload lives above the screen switch.** Today `Library` owns the
upload: its state, the `sendingRef` guard and the `AbortController` that its
unmount aborts. Opening an asset page unmounts `Library`, which would abort
a pending POST and drop its state, so Back would show an enabled Upload
button while the first upload may still run on the server. As the user
decided, the upload moves into a `useUpload(onSessionLost)` hook in a new
`web/src/upload.ts`, which `SignedIn` calls once, above the switch. The hook
owns the upload state (`idle`, `sending`, `refused`, `unconfirmed`), the
guard that sends no second request while one is in progress, a `saved`
counter that goes up after each 201 or unconfirmed answer, and the
`AbortController`. It aborts only when `SignedIn` unmounts: on sign-out or a
lost session, as the Library's unmount does today. `SignedIn` passes the
state, `send` and `saved` to `Library` as props. `Library` keeps its picker,
button, status lines and list. It adds `saved` to its list effect's
dependencies, so an upload that ended while the asset page showed is listed
when the Library shows again. `AssetPage` gets nothing from the hook and
shows no upload status. The existing `Library.test.tsx` upload tests move
to rendering through `SignedIn`, or pass the hook's values as props.

Each Library image and its name become one `Link` to `/assets/<id>`.

`SignedIn` renders the page as `<AssetPage key={id} id={id}>`. A history
jump can go straight from one asset page to another, with no Library in
between, and React would then keep the first page's state while the second
loads. The `key` gives each Asset id a fresh `AssetPage`, so the page shows
only `Loading the image…` until its own answer comes. The upload state lives
in `SignedIn`, above the key, so it survives the change.

`AssetPage` fetches `/api/assets/<id>` with an `AbortController`, as
`Library` does, and accepts the body only when `asset.id` equals the page's
id, `original_url` is `/api/assets/<id>/original`, and every facet has
string `name`, `label` and `unit` and a non-negative safe-integer `value`.
Core stores no count above `MAX_COUNT` (decision 2), so a valid answer always
passes this check.
Values are formatted with `Intl.NumberFormat("en-US")`, so the grouping is
commas in every locale. Its states follow the spec: loading, not found,
failed with Retry, loaded, and loaded with no facts. The image uses the
same broken-image message as the Library.

Using any other path inside the signed-in view (only possible on the Vite
dev server) shows the Library.

- *Alternative:* React Router. It is a new dependency for two screens and a
  back link, and its data APIs would compete with the fetch-and-abort
  pattern the Library uses.

### 9. The page's look: HTML drafts, then Penpot, then DESIGN.md

As the user chose (D9), the build step drafts the asset page as two or three
HTML files in `.workspace/asset-page/`, each using `web/src/styles/tokens.css`
and the Gallery layout family that DESIGN.md names for the asset viewer.
The user picks one. The chosen draft is drawn on a Penpot page named
`Options: asset-page` through the `visual-options` skill, after the tokens
are copied in. Then DESIGN.md gains an "Asset page" section under
Components, and only then is the page built. The task that does this stops
for the user's choice.

`pnpm design:lint` reads only `DESIGN.md`, so it passes before any of this
happens. The task is therefore done only when three things can be seen: the
task's line in `tasks.md` names the draft file the user picked; the Penpot
file has a page named `Options: asset-page` with that draft on it, which the
Penpot tools list (no project command reads Penpot); and `DESIGN.md` has an
`### Asset page` heading under `## Components`, which `grep` finds, and
still passes `pnpm design:lint`.

**Tokens this change adds:** none planned. The page can use existing tokens:
`color-text-body` and `color-text-secondary` for the name and labels,
`font-family-mono` with `font-size-14` and `line-height-20` for values,
`spacing-*` for gaps, and `color-border-hairline` for the broken-image edge.
If the chosen draft needs a token that does not exist, it is added to
DESIGN.md first, `pnpm tokens` regenerates `tokens.css`, and the Deviations
Ledger records why no existing token fit.

### ADR candidates

Decision 4's storage of facet values (costly to reverse once values exist)
is recorded in ADR-017, which replaces ADR-004's rule to store facets in
JSONB columns on the asset rows. Decision 2's registration check carries out
ADR-008's rule that one pack owns each schema; it needs no record of its own.
Decisions 5, 7 and 8 are reversible inside one story.

## Risks / Trade-offs

- [A server that stops during a run, or a database that fails while core
  records how a run ended, leaves an `operation_runs` row in `running` for
  good] → Accepted for this change: the Asset is saved and has no fact. A
  `running` row never has stored values, because the values and the
  `succeeded` status commit in one transaction (decision 3, step 4): when
  that commit landed, the row reads `succeeded`; when it did not, no value
  exists. The log line `file-size run not recorded …` says the outcome is
  unknown. Processing goals and their states (#527) own recovery; this
  change adds no retry.
- [The Asset's own commit answer is lost in `ingest`, so the upload answers
  an error and the file-size run never starts] → Out of scope: this is
  ingest's existing path from #521, which this change does not touch. The
  Asset shows `No facts are recorded for this image.`, as an image from
  before this change does. The case is raised on #532; recovering Assets
  that have no run belongs to processing goals (#527).
- [A stored object changed on disk to another file of the same size passes
  the size check] → Accepted: core does not hash the object again before
  each run, which would read every byte for every Operator. Detecting
  out-of-band changes belongs to reconciliation (ADR-003).
- [A build that changes a facet's label or unit without raising its version
  fails every run that writes it] → Intended: the stored schema version
  never changes, so the pack author must raise the version. The failure
  reason names the facet and the version.
- [Upload now waits for one extra run and database transaction] → The
  file-size Operator reads no bytes and does one small transaction. The
  route tests time nothing, but the browser journey's existing limits stay.
- [The image pack and core both know a file's size, so the fact repeats
  `asset_versions.byte_size`] → Intended (D3): the smallest fact that proves
  the contract.
- [Runtime refusals in the shared `AssetContext` (decision 1) find a wrong
  call later than a type error would] → The refusal fails the run with a
  reason, and contract tests cover both refusals.
- [A test seam on `Library` could reach production by mistake] → The
  builders are `#[doc(hidden)]` and only tests call them, as with
  `with_original_reader`.
- [Parallel stories #522 and #523 edit the image pack and contract] → This
  change only adds items to the contract and the pack, and renames nothing.

## Migration & Rollback

- **Deploy:** migration 5 runs at startup, in one transaction, as the
  others do. It creates three empty tables and changes no existing row.
  `scripts/instance-verify.sh` moves from four to five migrations.
- **Data:** no backfill (D2). Images saved before this change have no facts
  until a later story adds a way to run Operators over them.
- **Rollback:** migrations are forward-only, and an older binary refuses a
  database with five migrations, as `postgres-foundation` requires. To undo,
  ship a later change that stops writing facets; the tables can stay. Never
  edit migration records by hand.
