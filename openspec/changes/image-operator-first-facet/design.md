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
  and `facet_values` through SQL, as the existing tests read `assets`.
- **Existing seams reused:**
  - `plugin-abi`'s unit tests (`mod source_execution` pattern) for facet
    declarations, the namespace rule and Operator execution with in-memory
    bytes.
  - core's database tests (`core/src/ingest/tests.rs` pattern, with
    `test_support::TestDatabase`) for the executor's checks, run records and
    provenance, using test Operators registered through the public contract.
  - core's schema-snapshot test (`startup/tests.rs`) and upgrade test for
    migration 5.
  - `server/tests/web_root.rs` for the `/assets/<id>` answer.
  - Vitest with jsdom (`web/src/*.test.tsx`) for the asset page and
    navigation.
  - The browser journey (`web/e2e/upload.spec.ts`, run by `mise run e2e`) for
    upload → open the page → see the file size → reload.

## Scrutiny Tier & Blast Radius

- **Tier:** high. The change adds a public contract (Operator and facet
  declarations), a migration with three tables, and a trust boundary: which
  plugin may write which fact.
- **Blast radius:** `plugin-abi` (public API, ABI version 2), both packs,
  core's database schema and a new `operations` module, the upload route and
  the web-file fallback in the server, and the web app's signed-in view.
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
```

`FacetValue::Integer` takes any `i64`, so an Operator can propose -1 and core
can refuse it, as the spec's scenario needs. `OperatorError` mirrors
`SourceError` without `Refused`: `Unsupported` and `Failed(String)`.

A pack declares a schema with `Registry::declare_facet(schema)`. The host
registers each pack through a new `Registry::register_pack(&dyn Pack) ->
Result<(), RegistryError>`. It notes the pack's id, calls `Pack::register`,
and then checks what the pack declared: a name already declared, or a name
that does not start with `<pack id>.`, is refused with an error that names
the facet. `Pack::register` keeps its signature, so packs do not handle
errors. The server builds its Registry through `register_pack` for both
packs. A conflict among compiled-in packs is a build defect: `Library::new`
panics with the error's message, and a server unit test builds the
production Registry so the defect fails `mise run check`, not a start.

`ABI_VERSION` becomes 2. The astro `PlateSolveOperator` stub gains a version,
an empty `writes()`, and a `run` that returns `OperatorError::Unsupported`.

`Registry::execute_operator(id, candidate, log, cancelled)` runs the
Operator and returns its proposals with host-made provenance (`producer_id`,
`producer_version` from the registered Operator), or an `ExecutionError`.
It does not check grants or schemas: core does (decision 3).

### 3. Core's executor checks everything before it stores anything

A new core module, `operations`, holds
`run_operator(database, objects, registry, operator_id, version, cancelled)`.
In order:

1. Insert an `operation_runs` row with status `running`, the Operator's id
   and version, the Asset version id and `started_at = now()`, in its own
   transaction, so a failed run is still recorded.
2. On a blocking worker, open the version's object with
   `ObjectStore::open_object(content_hash)` and expose it as a `Candidate`
   through positioned reads. Run `Registry::execute_operator`. Log lines go
   to standard error as `operator <id>: <line>`, as Source lines do.
3. Check each proposal: a registered schema exists for the name (unknown);
   the name is in the Operator's `writes()` (grant); the value fits the
   schema's type (a `Count` is `>= 0`). Check cancellation once more.
4. In one transaction: upsert each used schema into `facet_schemas`, insert
   every value into `facet_values`, and set the run to `succeeded` with
   `ended_at`. On any failure from steps 2–4, set the run to `failed` with
   its reason and `ended_at`, and store no value.

A panic in the Operator ends the blocking task with a join error; core
records `failed` with the reason `the operator stopped`. An `Unsupported` or
`Failed(reason)` result is recorded with that reason. A refused proposal's
reason names the facet, for example `facet test.width is not granted to
test.op`.

Core takes schemas from the Registry at run time, so it never trusts what a
pack wrote into the database earlier. The `facet_schemas` rows are a copy for
integrity (a value always has a schema row) and for reading labels and units
without the Registry.

### 4. The tables: migration 5

```sql
CREATE TABLE public.operation_runs (
    id UUID PRIMARY KEY,
    operator_id TEXT NOT NULL CHECK (operator_id <> ''),
    operator_version TEXT NOT NULL CHECK (operator_version <> ''),
    asset_version_id UUID NOT NULL REFERENCES public.asset_versions (id),
    status TEXT NOT NULL CHECK (status IN ('running', 'succeeded', 'failed')),
    reason TEXT CHECK (reason <> ''),
    started_at TIMESTAMPTZ NOT NULL,
    ended_at TIMESTAMPTZ,
    CHECK ((status = 'failed') = (reason IS NOT NULL)),
    CHECK ((status = 'running') = (ended_at IS NULL)),
    CHECK (ended_at >= started_at)
);

CREATE TABLE public.facet_schemas (
    name TEXT PRIMARY KEY CHECK (name ~ '^[a-z0-9-]+(\.[a-z0-9-]+)+$'),
    owner_pack TEXT NOT NULL CHECK (owner_pack <> ''),
    label TEXT NOT NULL CHECK (label <> ''),
    value_type TEXT NOT NULL CHECK (value_type IN ('count')),
    unit TEXT NOT NULL,
    scope TEXT NOT NULL CHECK (scope IN ('asset', 'asset_version')),
    version INTEGER NOT NULL CHECK (version >= 1)
);

CREATE TABLE public.facet_values (
    id UUID PRIMARY KEY,
    facet TEXT NOT NULL REFERENCES public.facet_schemas (name),
    asset_id UUID REFERENCES public.assets (id),
    asset_version_id UUID REFERENCES public.asset_versions (id),
    value JSONB NOT NULL CHECK (jsonb_typeof(value) <> 'null'),
    producer_id TEXT NOT NULL CHECK (producer_id <> ''),
    producer_version TEXT NOT NULL CHECK (producer_version <> ''),
    run_id UUID NOT NULL REFERENCES public.operation_runs (id),
    schema_version INTEGER NOT NULL CHECK (schema_version >= 1),
    written_at TIMESTAMPTZ NOT NULL,
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
  schema, its Asset or Asset version, and its run. A file size is stored as a
  bare JSON number, such as `1234567`. A new value type needs no migration.
- **Core checks the value's type, not PostgreSQL.** A row check cannot read
  the schema's `value_type` in another table, so the database only refuses
  JSON `null`. Core checks every value against the Registry's schema before it
  stores it (decision 3), and only core writes the table (ADR-008).
- **Writing and reading the column.** The workspace's `sqlx` gains its `json`
  feature, so core binds and reads `serde_json::Value`. Core turns
  `FacetValue::Integer(n)` into a JSON number, and reads a `count` value back
  with `as_i64()`; a stored value that is not a whole number of zero or more
  is a database error, not a fact.
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
- **`facet_schemas` rows are written when a value first needs them**, inside
  the run's transaction, with `ON CONFLICT (name) DO UPDATE` from the running
  build's declaration. Startup writes nothing new, so the readiness, restore
  and "nothing is stored" checks keep their behavior, and the upgrade
  scenario's tables are empty. `schema_version` on each value keeps the
  version that applied when it was written.
- **No unique key on (facet, version).** ADR-008 keeps every observation; a
  rule for which value is current is not part of this change.
- Mint all ids in core with `Uuid::new_v4()`, as `assets.rs` does.
- Regenerate `core/schema/5.json` with `mise run db-schema` and add it to
  `baselines()`.

### 5. Upload runs the file-size Operator after the commit

After `ingest()` returns the saved record, the upload handler calls
`tokio::spawn` with `run_operator(…, FILE_SIZE, …)` and awaits the join
handle. A spawned task keeps running when its handle is dropped, so a client
that goes away does not stop the run. The run gets its own cancel flag,
never set by the request; `CancelOnDrop` keeps guarding only the ingest
before the commit.

Whatever the run's result, the handler answers 201 with the existing body.
A failed run, a refused proposal, a failure to record the run, or a join
error writes one line through the server's `Log`:
`file-size run failed for asset <id>: <error>`.

The image pack exports `FILE_SIZE` (the Operator id `image.measure-file-size`)
and `FILE_SIZE_FACET` (`image.file-size`). The Operator proposes
`context.candidate_size()` as the value. It reads no bytes: the size is the
fact, and `ObjectStore::open_object` already gives core the stored size.

For tests, `Library` gains two `#[doc(hidden)]` builders, like
`with_original_reader`: `with_registry(Registry)` and
`with_upload_operators(Vec<String>)`. A test registers the image pack plus a
test pack whose Operator fails, blocks on a channel, or proposes a refused
value, and points upload at it.

### 6. `GET /api/assets/{id}` returns the Asset with its facets

The route sits in `assets::routes`, inside the session guard. It parses the
id like the original route (404 for a malformed id), reads the Asset, then
calls a new `Database::facets(asset_id, current_version_id)`. That query
joins `facet_values` to `facet_schemas` for values whose `asset_id` is the
Asset or whose `asset_version_id` is its current version, ordered by facet
name, then `written_at`, then id. The body is the list entry plus
`"facets": [{"name", "label", "value", "unit"}]`. A database error is 503
with the library's fixed message.

### 7. The server answers `/assets/<id>` with `index.html`

`with_web_root` wraps its file fallback in one small service. When the path
matches `^/assets/[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$`,
it answers with the same `ServeFile` for `index.html` that `/` uses.
Every other path goes to `ServeDir` unchanged, so Vite's `/assets/index-….js`
files, a tester's `assets/index.html`, and every 404 keep working. The match
is on the exact text: an uppercase id or a trailing slash is not an asset
page address. Vite's output folder stays where it is.

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

`Access.tsx`'s signed-in branch picks the screen from the path:
`/assets/<id>` (the same pattern as decision 7) shows `AssetPage`; any other
path shows `Library`. Both get `onSessionLost` and the account items
(username and Sign out) as children for the top bar, so the bar looks the
same on both screens. When the session ends, Access shows the sign-in form
and keeps the path, so signing in again shows the same screen.

Each Library image and its name become one `Link` to `/assets/<id>`.

`AssetPage` fetches `/api/assets/<id>` with an `AbortController`, as
`Library` does, and accepts the body only when `asset.id` equals the page's
id, `original_url` is `/api/assets/<id>/original`, and every facet has
string `name`, `label` and `unit` and a non-negative safe-integer `value`.
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

- [A server that stops during a run leaves an `operation_runs` row in
  `running` for good] → Accepted for this change: the Asset is saved and only
  the fact is missing. Processing goals and their states (#527) own
  recovery; this change adds no retry.
- [Upload now waits for one extra run and database transaction] → The
  file-size Operator reads no bytes and does one small transaction. The
  route tests time nothing, but the browser journey's existing limits stay.
- [`facet_schemas` rows follow the newest build's label and unit, so an
  older value can show under a changed label] → Each value keeps its
  `schema_version`; a breaking change to a facet needs a new version
  (ADR-008). There is one schema at version 1 today.
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
