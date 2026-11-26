<!-- Plain language: Help the next implementer find the approach, understand the choices and act on the plan. Explain necessary technical terms. Put each risk beside its mitigation and each open question beside the decision it affects. -->

## Context

This change is stacked on #547 (`image-format-and-dimensions`), in the form
that #547 took after its review: #547 records no format facet and adds no
choice type. The asset page shows the format from the version's
`media_type`, as `Format PNG`, and `GET /api/assets/{id}` returns
`media_type`.

- **Contract.** `server/crates/plugin-abi/src/lib.rs` is at `ABI_VERSION` 2.
  `FacetType` has only `Count`. `AssetContext` has a private `RunKind`.
  `propose_asset` works only in a Source run, and `propose_facet` only in an
  Operator run. A broken rule sets `broken`, which turns the run into
  `ExecutionError::Failed`. An Operator learns nothing about its input but
  its bytes and their size. `Registry::execute_operator(id, candidate, log,
  cancelled)` returns `FacetProposals`, which holds the producer and the
  grants from the Operator's registration. `RegisteredOperator` keeps `id`,
  `version` and `writes` from registration, and `register_pack` refuses an
  Operator whose id or version core cannot store.
- **Core runs.** `server/crates/core/src/operations.rs` `run_operator` reads
  the version's Asset, hash and size, inserts a `running` row, runs the
  Operator in `spawn_blocking` over `ObjectBytes`, checks the proposals in
  `check`, then `store` writes the schema rows, the values and `succeeded`
  in one transaction. `RunOutcome` is `Succeeded` or `Failed(reason)`.
  `record_failure` updates only a `running` row. A panic in the blocking
  task becomes the reason `the operator stopped`.
- **Assets.** `server/crates/core/src/assets.rs` selects every Asset joined to
  its current version, `media_type` included. `find_or_create_asset` takes
  an advisory lock on the hash, matches any version's `content_hash`, and
  inserts an Asset and its first version with a deferred foreign key.
  `asset_versions` has `source_id` and `source_version`, both `NOT NULL`.
- **Storage.** `ObjectStore::begin()` gives a `Staging` that refuses more
  than `MAX_UPLOAD_BYTES` (100 MiB) with `StageError::TooLarge`.
  `finish()` hashes the bytes into a `Staged`, and `publish()` makes the
  object durable before any record (ADR-011). A dropped `Staging` or `Staged`
  removes its temporary file.
- **Migrations.** `server/crates/core/migrations/` holds 1 to 5. Each has a
  generated baseline `server/crates/core/schema/<n>.json`, made by
  `mise run db-schema` and listed in `baselines()` in
  `core/src/database/startup.rs`. Startup tests count the migrations, and
  `scripts/instance-verify.sh` expects five.
- **Image pack.** `server/crates/packs/image/src/lib.rs` holds a private
  `Format` enum (PNG and JPEG), the `head()` function that reads a fixed
  8 bytes, the manual-upload Source, `image.measure-file-size`, and the
  `UNSUPPORTED` text `only PNG and JPEG images are supported`.
  `src/header.rs` holds `image.read-header`, which reads the signature again
  to pick its decoder, and `CandidateReader`, a `BufRead + Seek` view of the
  candidate built on 64 KiB `read_candidate` calls, with the 8 MiB JPEG read
  limit. The pack depends on `plugin-abi`, `png` 0.18 and `zune-jpeg` 0.5.
  `tests/memory.rs` is its own test binary, with a counting allocator that
  counts per thread.
- **Server.** `server/crates/server/src/assets.rs` runs `upload_operators`
  (`FILE_SIZE`, `READ_HEADER`) in order after a new Asset is saved.
  `entry()` builds the JSON entry, and `read` adds `media_type`. `original()`
  streams an object with `nosniff` and `no-store`, and ends the body early
  when a read fails. A Source's refusal, `IngestError::Refused(reason)`,
  becomes `UploadError::Unsupported`, which drops the reason and answers 415
  with the fixed text `unsupported format: only PNG and JPEG images are
  supported`.
- **Web.** `web/src/entry.ts` `toEntry` accepts an entry only when
  `original_url` is exactly `/api/assets/<id>/original`. `Library.tsx` and
  `AssetPage.tsx` show `originalUrl` in an `<img>`, with a broken-image
  message on `error`. `AssetPage.tsx` maps `image/png` to `PNG` and
  `image/jpeg` to `JPEG`, and shows any other media type as it is.
  `upload.ts` turns each refusal status into fixed words, and says the
  server's words never reach the screen; its 415 and 400 texts name PNG and
  JPEG. `Library.tsx` sets the picker's `accept` to `image/png,image/jpeg`
  and shows `No images yet. Upload a PNG or JPEG to start.`
- **Crates checked for this design.** `tiff` 0.11 decodes classic and
  BigTIFF in both byte orders, with optional `lzw` and `deflate` features;
  PackBits needs no feature. Its `Limits` caps the decoding buffer.
  `jpeg-encoder` 0.7 writes a baseline JPEG from an `ImageBuffer` trait that
  asks for one row at a time, and it takes width and height as `u16`.
  zune-jpeg keeps whether a JPEG is progressive private.

## Goals / Non-Goals

**Goals:**

- One general way for an Operator to output a derived asset with lineage,
  used first by previews. Later outputs (thumbnails, stacks) add a role, not
  a new mechanism.
- A preview Operator in `packs/image` that keeps each decode within 512 MiB,
  including pixels and decoder scratch space.
- The image pack is the one owner of its formats: adding a format touches
  one module and one table entry, and the server and the web repeat no list.
- An Operator learns a version's format from its media type, and runs only
  on versions it says it accepts.
- The web interface shows a preview with no change to its layout or tokens.

**Non-Goals:**

- Backfilling previews, or a way to make one by hand (D8).
- Thumbnails, a tile pyramid or resizing.
- Brightening or stretching (D5), colour management, or applying EXIF or
  TIFF orientation.
- A general lineage API or a lineage view. Only the preview link is read.
- A format facet or a choice value type (D16).
- An `accepts` Selector over facets, kinds or Collections, or a way for an
  Operator to accept a derived role (D15).

## Test Seams

Each layer has a seam that already exists. Every one is used, because each
crate's behavior is checked at its own boundary.

- **Primary test seam:** the server's HTTP tests in
  `server/crates/server/tests/assets.rs`, which upload through the router
  against PostgreSQL and read the list, the Asset, the original and now the
  preview. They prove the upload journey, the 415 reason, `preview_url`,
  the preview route, originals-only listing and repeat-upload matching.
- **Existing seams reused:**
  - `plugin-abi` unit tests in `src/lib.rs`, for the accepted media types,
    the context's media type, and the context's rules on proposing a
    preview and writing its bytes.
  - Core's run tests under `core/src/operations/` with the `fixture.rs`
    database and `Scripted` Operators, for runs that core does not accept,
    core's checks, the derived-asset commit, lineage, cancellation and the
    newest preview.
  - Core's startup tests in `core/src/database/startup/tests.rs`, for
    migration 6.
  - The image pack's tests in `packs/image/tests/`, which run
    `Registry::execute_operator` over in-memory bytes with a media type.
    New files `tiff_header.rs` and `preview.rs` follow `header.rs`.
  - Unit tests inside each format module of the image pack, for that
    format's read limits and memory peaks (decision 6).
  - The web unit tests `Library.test.tsx`, `AssetPage.test.tsx`,
    `upload.test.ts`, `SignedIn.test.tsx` and `Access.test.tsx`.

## Scrutiny Tier & Blast Radius

- **Tier:** high
- **Blast radius:** the public plugin contract (ABI 2 to 3), a database
  migration, core's run and asset queries, the image pack and its packaged
  helper, the upload and
  Asset routes and two web screens. Decoders read files that people upload,
  so memory and failure handling are part of the trust boundary. Every
  Operator must now declare the media types it accepts, so an Operator with
  a wrong list silently stops running.

## Decisions

### 1. The contract: an Operator declares previews and streams their bytes

- `Operator` gains a provided method `fn makes_previews(&self) -> bool`,
  which returns `false`. The Registry reads it once, at registration, into
  `RegisteredOperator`, as it reads `writes`.
- `AssetContext` gains, for Operator runs only:
  - `propose_preview(&mut self, proposal: AssetProposal) -> Result<(), ContextError>`,
    reusing `AssetProposal`'s kind and media-type checks;
  - `write_preview(&mut self, bytes: &[u8]) -> Result<(), ContextError>`.
  A second proposal, or a write before a proposal, sets `broken` and returns
  an error. A Source run gets `ContextError::NotAllowed` and `broken`, as an
  Operator calling `propose_asset` does today. A new
  `ContextError::Write` reports a write the host refused.
- The host supplies the destination: a new public trait
  `Output { fn write(&mut self, bytes: &[u8]) -> std::io::Result<()>; }`.
  `execute_operator` takes `output: &mut dyn Output` after the media type
  (decision 2). `execute_source` gains no output.
- `FacetProposals` becomes `OperatorProposals`. It keeps `producer_id`,
  `producer_version`, `grants` and `proposals`, and adds `makes_previews`
  (from registration), `preview: Option<AssetProposal>` and
  `preview_bytes: u64` (the count the context passed on).
- `ABI_VERSION` becomes 3, with one doc line for decisions 1 and 2.

*Alternatives:* the Operator returns the preview as a `Vec<u8>`. That holds
the whole JPEG in memory as well as the pixels, and puts no bound on it until
the run ends. A general `propose_output(role, …)` names roles before a second
one exists; `propose_preview` is narrower and can grow into it with a later
ABI. This decision is recorded in ADR-018, with decision 4.

### 2. The contract: an Operator sees the media type and declares what it accepts

- `Operator` gains a required method `fn accepts(&self) -> &[&str]`: the
  media types of originals' versions it reads, such as `image/png`. It is
  required, not provided, so every Operator says what it reads. A default
  of every media type would let an existing Operator run on anything; a
  default of none would make it silently never run.
- The Registry reads `accepts` once, at registration, into
  `RegisteredOperator`, which gains `accepts(&self) -> &[String]` and
  `accepts_media_type(&self, media_type: &str) -> bool`, an exact match.
  `register_pack` refuses an Operator, with
  `RegistryError::InvalidOperator`, whose list is empty (`it accepts no
  media type`) or holds an entry that is not a well-formed media type, by
  the rule `AssetProposal` already uses (`it accepts a media type that is
  not well formed`).
- `AssetContext` gains `media_type(&self) -> Option<&str>`. In an Operator
  run it is the media type core recorded for the version. In a Source run it
  is `None`: the Source's candidate has no recorded media type, because
  recognizing it is the Source's job.
- `execute_operator(id, candidate, media_type, output, log, cancelled)`
  takes the version's media type as `&str` after the candidate. When the
  Operator does not accept it, `execute_operator` returns a new
  `ExecutionError::NotAccepted` without running the Operator. So an
  Operator run through the Registry never sees a media type it did not
  list, whichever host runs it.
- The image pack's three Operators take their lists from the `FORMATS`
  table (decision 6), so each accepts `image/png`, `image/jpeg` and
  `image/tiff`.

*Alternatives:* each Operator reads the signature and fails on a format it
does not handle, as the header Operator does today. Every Operator would
repeat the Source's work, and every run over an unhandled format would be a
failed run. The caller, such as the upload, could choose the Operators for
each media type; what a pack can read would then live in the server. This
decision is recorded in ADR-019.

### 3. Core runs an Operator only on a version it accepts

- `run_operator` reads the version's `media_type` and its Asset's `origin`
  (decision 4) with its hash and size, in the query it already makes. It
  runs the Operator only when the Asset is an original and the Operator
  accepts the media type. Otherwise it returns a new outcome,
  `RunOutcome::NotAccepted`, before it inserts the run row, so no run is
  recorded and no failure is logged.
- **How a preview is kept out (D9).** An Operator's list names media types
  of originals only. A preview's version belongs to a derived Asset, whose
  role is `preview`, and the contract gives no way to accept that role yet.
  So the header Operator, which accepts `image/jpeg`, never runs on a JPEG
  preview, and the preview Operator never makes a preview of a preview. The
  rule needs no case for previews in the server: it holds for any caller
  of `run_operator`, including a later reconciler. Core tests run a
  `Scripted` Operator that accepts `image/jpeg` over a preview's version
  and assert `NotAccepted` and no run row. The server's upload test asserts
  that the preview's version has no run.
- The upload's `save` treats `NotAccepted` as nothing to log. Every upload
  Operator accepts every format the Source recognizes, because all of them
  take their lists from one table.

*Alternatives:* skip derived versions in the upload's loop. That is a rule
for one caller, which the next caller would miss. Record a run that is
`failed` with a reason such as `not accepted`. It fills the run history with
runs that never ran, which #527's view of processing failures would show.

### 4. Core records a derived asset and its lineage in the run's transaction

- Migration `6_derived_assets.sql`:
  - `public.assets` gains `origin TEXT NOT NULL DEFAULT 'source'
    CHECK (origin IN ('source', 'derived'))`. The default fills existing
    rows, then the migration drops it, so every insert names the origin.
  - New table `public.asset_lineage (output_version_id, input_version_id,
    role, run_id)`: both versions reference `asset_versions (id)`, `run_id`
    references `operation_runs (id)`, `role TEXT NOT NULL CHECK (role IN
    ('preview'))`, primary key `(output_version_id, input_version_id)`, a
    check that the two versions differ, and an index on
    `(input_version_id, role)`.
- `run_operator` gives the Operator a `StagingOutput`. On the first write it
  calls `ObjectStore::begin()`. It passes each write to `Staging::write` and
  remembers the first staging error across subsequent writes, including an
  error starting staging. A failed output cannot be published, even if the
  Operator claims success. All of this runs in the existing `spawn_blocking`
  task.
- After the run, core checks the preview with the facet proposals, in
  `check`. The run fails, and the staging is dropped, when: a preview came
  from an Operator with `makes_previews` false (`operator <id> does not make
  previews`); the context reported more than one proposal or a write with
  none (the context's own reason); the media type is not `image/jpeg` or
  `image/png` (`a preview cannot be <media type>`); no byte was written
  (`the preview is empty`); or staging refused a write
  (`the preview is larger than 100 MiB`, which takes the place of the
  Operator's reason); or another staging error occurred (`the preview could
  not be stored`).
- After the cancellation check, core calls `finish()` and `publish()` on a
  blocking worker. After publication, check cancellation again before
  starting `store`. Once that transaction starts, it decides the outcome.
  A cancellation or transaction failure after publication can leave an
  unreferenced object. Then `store` gains, inside its transaction and before the
  run is marked `succeeded`: read the input version's Asset name; insert the
  derived Asset (`origin` `derived`, the proposed kind, that name); insert
  its version (core's hash and size, the proposed media type, the Operator's
  id and version in `source_id` and `source_version`); insert the lineage row
  with role `preview` and the run id. A failed publish records the run
  `failed` with `the preview could not be stored`.
- `source_id` and `source_version` keep their names and now hold the
  producer: a Source for an original, an Operator for a derived asset.
  `origin` says which.

*Alternatives:* a side table of "renditions" attached to a version, with no
Asset. It is simpler to serve, but ADR-003 says derived output is a separate
Asset, and the glossary's Preview is a derived asset. A new version of the
original Asset is ruled out by ADR-003: the preview is not the original's
bytes. This decision is recorded in ADR-018.

### 5. Asset queries cover originals only, and find the newest preview

- `select()` in `assets.rs` gains `WHERE a.origin = 'source'` and a
  `preview` column from a lateral subquery: the derived version linked to the
  current version with role `preview`, by the run with the latest
  `ended_at`, then the highest run id. The success update uses
  `greatest(clock_timestamp(), started_at)` to record when the result is
  marked succeeded. This orders by the recorded time. `AssetRecord` gains
  `has_preview: bool`. `Database::assets()` and `Database::asset(id)` both
  use it, so a derived id reads as missing.
- A new `Database::preview(asset_id) -> Option<VersionRecord>` gives the
  preview version of an original's current version, for the preview route.
- `find_or_create_asset` matches `content_hash` only on versions whose Asset
  has `origin = 'source'`. The advisory lock stays as it is. Its insert names
  `origin` `source`.

### 6. The image pack: one module per format, driven by one table

The pack's source files become:

| File | Holds |
|---|---|
| `src/lib.rs` | `ImagePack`, the public ids, the facet schemas, the manual-upload Source and the file-size Operator. It registers every Operator with the media types from `FORMATS`. |
| `src/io.rs` | `CandidateReader`, moved from `header.rs` with no format in it, and `head(context, len)`, which reads up to `len` leading bytes. |
| `src/formats/mod.rs` | The `ImageFormat` trait, the `FORMATS` table and the functions built on it. |
| `src/formats/png.rs`, `jpeg.rs`, `tiff.rs` | One format each, with its read limits and its own unit tests. `tiff.rs` has a private submodule, `tiff/ifd.rs`, for the directory reader (decision 7). |
| `src/header.rs` | The header Operator, with no format in it. |
| `src/preview.rs` | The preview Operator, the shared pixel buffer, the shared limits, the 8-bit conversion and the JPEG encoder (decision 8). |
| `src/testing.rs` | Test-only: the counting allocator and an in-memory candidate. |

- **The trait.** `ImageFormat` is internal to the pack (`pub(crate)`), so
  it is not part of the plugin contract:

  ```rust
  pub(crate) trait ImageFormat: Sync {
      /// The name in failure reasons and the refusal, such as `PNG`.
      fn name(&self) -> &'static str;
      /// The media type the Source records, such as `image/png`.
      fn media_type(&self) -> &'static str;
      /// How many leading bytes `matches` needs.
      fn probe_len(&self) -> usize;
      /// Whether `head`, the candidate's first bytes, starts with this
      /// format's signature. `head` is shorter than `probe_len` only when
      /// the candidate is.
      fn matches(&self, head: &[u8]) -> bool;
      /// The width and height the header stores, read under this
      /// format's own read limits, or `None` when they cannot be read.
      fn dimensions(&self, reader: &mut CandidateReader<'_, '_>) -> Option<(u32, u32)>;
      /// Render the first image for its preview: read the header, check
      /// it against the shared limits, then decode its pixels.
      fn render_preview(&self, reader: &mut CandidateReader<'_, '_>)
          -> Result<Pixels, PreviewError>;
  }
  ```

- **The table.** `pub(crate) static FORMATS: &[&dyn ImageFormat] =
  &[&png::Png, &jpeg::Jpeg, &tiff::Tiff];`. The functions built on it:
  - `probe_len()`: the largest `probe_len` in the table, 8 today (PNG).
    The Source reads that many bytes, so a format with a longer signature
    raises it by itself.
  - `recognize(head)`: the first format whose `matches` is true.
  - `by_media_type(media_type)`: the format with that media type.
  - `media_types()`: every format's media type, in table order. It is each
    Operator's `accepts` list.
  - `refusal()`: `only ` + the names joined as `PNG, JPEG and TIFF` +
    ` images are supported`. The pack's public `UNSUPPORTED` constant
    becomes a public function, `unsupported()`, that returns it, so tests
    can read it.
  A unit test in `formats/mod.rs` checks that no two formats share a media
  type, that no format's signature matches another's, and the exact
  refusal text.
- **Recognition.** The Source reads `probe_len()` bytes with `head`, then
  `recognize`, and refuses with `refusal()`. It proposes the format's media
  type.
- **The header Operator** (`header.rs`) picks its format with
  `by_media_type(context.media_type())`, never by the signature. It runs
  `format.dimensions` through a `CandidateReader`. It fails with
  `the <name> header could not be read` when that gives nothing or a zero
  side. A media type with no format fails with
  `the image format is not recognized`; the Registry never runs it with one
  (decision 2), so this is only a guard. A version whose bytes do not match
  its media type fails as a header of that format.
- **A failed read is a storage problem.** `CandidateReader` records when a
  `read_candidate` call fails. When it did, the header and preview
  Operators fail with `the stored file could not be read`, whatever the
  decoder says. This widens #547's rule, which covered only the read of the
  signature, to every read, now that the Operator no longer reads the
  signature first.
- **Each format's own tests.** The counting allocator moves from
  `tests/memory.rs` to `src/testing.rs`, installed as the
  `#[global_allocator]` of the pack's unit-test build only
  (`#[cfg(test)]`). It counts per thread, so unit tests that run at the same
  time do not mix their counts. Each format module has a `#[cfg(test)] mod
  tests` with its read-limit and memory tests: the PNG of 200,000 text
  chunks moves to `png.rs`, the JPEG of 19-byte ICC segments and the 8 MiB
  read-limit cases move to `jpeg.rs`, and the TIFF directory caps go to
  `tiff.rs`. The preview's memory cases join them (decision 8).
  `tests/memory.rs` is removed. Tests of the public behavior stay in
  `tests/`.

*Alternatives:* keep the `Format` enum with a `match` per operation. Each
new format then edits every `match` in several files, and the refusal text
and the Operators' lists are written by hand. A public format trait in
`plugin-abi`, so that other packs add formats to the image pack. That is a
contract nobody needs yet; `packs/astro` will recognize FITS in its own
Source.

### 7. TIFF: signatures, one small directory reader, and its header

- `tiff.rs` names `TIFF`, with media type `image/tiff` and a probe of 4
  bytes. Its signatures are `II*\0`, `MM\0*`, `II+\0` and `MM\0+`.
- Its private submodule `tiff/ifd.rs` reads the 8-byte (classic) or
  16-byte (BigTIFF) header and the first image directory through
  `CandidateReader`. It returns the values of the tags it needs: width
  (256), length (257), bits per sample (258), compression (259),
  photometric interpretation (262), samples per pixel (277), planar
  configuration (284), predictor (317), extra samples (338) and sample
  format (339) in preview mode. It reads at most 4,096 directory entries.
  The 64-entry limit applies to arrays of sample properties. The helper
  checks layout arrays separately as decision 8 describes.
- In dimensions mode it reads only width and height values.

*Alternative:* `tiff::decoder::Decoder::new` for the header. It reads the
first directory too, but it refuses some color types and compressions while
it sets up, and the header run must succeed for every TIFF (D4).

### 8. The preview Operator runs a memory-capped decoder helper

- **Boundary.** The image pack owns format recognition, decoder preflight,
  pixel conversion and JPEG encoding. The server wires a local helper runner
  into the image pack at registration. The preview Operator proposes its
  output and uses the runner with its Asset context. The runner reads the
  candidate and writes the result through that context. Core and the public
  plugin ABI know nothing about the helper (ADR-020).
- **Input and output.** The runner copies at most 100 MiB to one anonymous
  temporary file using a fixed-size buffer. The helper receives the rewound
  file as standard input and the recorded media type as an argument. It
  receives no storage path or inherited environment. It writes JPEG chunks
  to stdout. The runner forwards them through a bounded handoff into
  the Asset context, where storage writes can block. Discard stderr.
- **Child ownership and deadline.** A supervisor task owns the child,
  the liveness pipe's only write end, and the preview permit. It owns
  neither the stdout read loop nor the Asset context. Start a 120-second
  deadline at spawn. On deadline or a cancellation or output-failure
  notification, kill and reap the child from the supervisor task even
  if the runner is blocked in storage. On normal exit, reap it and
  report the exit status. A parent exit makes the helper's liveness
  reader see EOF. Release the preview permit only after reaping, before
  waiting for any outstanding staging write to settle. A failed helper's
  bytes cannot complete a proposal. Surface a child timeout once a
  blocked staging call returns, discard the staging, and leave the
  original available. Other previews can run while that write is blocked.
- **Packaging.** Build and ship the helper beside the server. Resolve it
  from the server executable's directory. The dev launcher builds both
  binaries before starting the server. Document the same prerequisite for
  manual builds. Run the database-free helper integration suite on
  `ubuntu-latest` and `macos-15` in CI's `preview-worker` job. Run a
  preview smoke test as the shipped container user in `instance-verify`.
  Run `scripts/dev-supervisor-check.sh` after changing the launch command.
- **Decoder memory ceiling.** Request a 2 MiB decoder-thread stack.
  Before reading input, get the stack size from the operating system and
  charge the full reported size, including platform rounding. Refuse
  startup if that size is unavailable, zero or greater than 4 MiB.
  Subtract it from 512 MiB and round the remainder down to an OS page
  boundary. Map that remainder once for the fixed-capacity heap.
  Refuse to enter the decoder if the mapping fails. Every Rust allocation
  on that thread, including zeroed allocations and growth, uses
  `linked_list_allocator::Heap` over the mapping. Free-list records,
  alignment and simultaneous old and new buffers occupy the same mapping.
  There is no fallback from the mapping to the system allocator.
  Allocation refusal exits with the budget status without allocating.
  All byte-dependent work, including metadata parsing, decoder
  construction, conversions, and encoding, stays on this one thread.
  The file and output buffers used by those operations are also charged
  to it. The bootstrap and liveness watcher use fixed-size state and never
  receive candidate bytes. Executable mappings and this fixed supervisor
  state are process overhead outside the decoder working-memory budget.
  Keep codecs and their dependencies on the Rust allocator with no native
  codec, worker threads or separate mappings. Review that property when
  dependencies or codec features change.
- **Preflight.** From bounded headers, check the dimensions first, with
  each side at most 65,500. For TIFF, check layout eligibility second.
  Then use checked arithmetic to ensure the minimum known pixel allocation
  fits the available allocation budget. Enter pixel decoding only after
  these checks. The helper's budget also covers allocations after preflight,
  including the TIFF constructor, predictor scratch, multiple-scan
  baseline and progressive JPEGs and metadata between scans. Set PNG's
  resource limit and TIFF's decoding and intermediate buffer limits to
  the arena capacity so codec defaults do not reject supported layouts.
  The arena enforces the combined limit. A codec limit refusal maps to the
  budget reason. An operating-system failure outside an explicit budget
  refusal maps to the worker failure reason.
- **TIFF.** Read only width and height for the host header Operator.
  The helper reads preview eligibility tags separately, with a small
  limit on arrays of sample properties. Check strip and tile offset and
  byte-count array lengths against geometry using checked arithmetic.
  These layout arrays use the memory budget and can exceed 64 entries.
  Accept absent Predictor or `1` for no prediction, `2` for unsigned
  integers and `3` for floats where compatible with the compression.
  Accept white-is-zero along with black-is-zero. Let the TIFF decoder
  invert white-is-zero pixels before scaling to 8-bit. Use `tiff` with
  only its LZW and Deflate features enabled.
- **JPEG.** Keep the 8 MiB bound for the frame-header read and then rewind
  to the full input. Require the first image's EOI, coverage of each
  baseline frame component, and complete entropy data for the blocks
  each scan declares. The pixel decoder must report premature scan
  termination, even when a valid EOI follows. The codec selection
  remains open. zune-jpeg 0.5.15's strict setting still allows synthetic
  bits after a marker, and jpeg-decoder 0.3.2 also pads its lookahead.
  A marker walk cannot certify that all blocks were decoded.
- **PNG.** Keep `EXPAND` with 16-bit samples and call `Reader::finish()`
  after decoding so the file must reach IEND.
- **Preview output.** Decode into one `Pixels` buffer. `Rows` maps each
  row to 8-bit Luma or RGB according to the spec. Encode a quality-90
  baseline JPEG with 4:4:4 sampling and standard Huffman tables. Stream it
  without buffering a whole JPEG. An unexpected helper exit is a preview
  failure and never completes the proposal. Separate exit statuses identify
  a decode budget rejection, unsupported TIFF, oversized dimensions and
  invalid image data. Use the spec's failure reason for each status.
  Report the first host failure that caused shutdown even if the process
  later exits with a signal. A candidate read error records
  `the stored file could not be read`, deadline records
  `the preview decoder timed out`, and other unexpected exits record
  `the preview decoder stopped`. Cancellation and output write failure
  preserve their existing context and core reasons.
- **Tests.** In the worker integration suite, test with a smaller mapping
  and assert the allocator refuses an oversized zeroed allocation and
  overlapping growth. Check stack sizes on both platforms. Test real
  budget exhaustion during TIFF construction and decode. Exercise a
  helper that stalls before output, closes stdout and stalls, or fails
  after returning some bytes. Block a staging write while the helper
  is still running, verify the supervisor reaps the child at deadline,
  then start a second preview before unblocking the first write.
  Verify no failed output is published. Cover white-is-zero,
  floating-point strips and padded tiles, more than 64 tiles, an
  incompatible predictor, and a valid TIFF whose pixels exceed 256 MiB.
  Test JPEG separate component scans, inter-scan metadata, missing EOI and
  EOI before all baseline components or before all blocks in a final scan.
  Test PNG with complete pixels but
  missing IEND. For a preflight refusal, instrument entry to pixel
  decoding and use an input with actual pixel data.

### 9. The server runs one preview at a time and passes the Source's refusal on

- `upload_operators` becomes `FILE_SIZE`, `READ_HEADER`, `MAKE_PREVIEW`.
- The preview runner shares one `tokio::sync::Semaphore` across uploads.
  The supervisor acquires an owned permit before launching each helper
  and releases it after reaping. The permit covers the child lifetime.
  The other Operators and final storage writes do not wait for it.
  An overlapping-upload test holds the first helper, checks the second
  waits to launch and verifies it can launch after the first is reaped.
- `save` returns the record with `has_preview` set to whether the preview run
  succeeded. Core commits the lineage in the same transaction that records
  `succeeded`, so no second read is needed. `save` logs nothing for
  `RunOutcome::NotAccepted`.
- `entry()` adds `preview_url`: `/api/assets/<id>/preview`, or `null`.
- The route `GET /api/assets/{id}/preview` shares one helper with
  `original()` that streams an object with its media type, `nosniff`,
  `no-store`, and the early end on a failed read.
- **The refusal is the Source's (D14).** `UploadError::Unsupported` holds
  the reason from `IngestError::Refused(reason)`. Its 415 body is
  `{"error": "unsupported format: <reason>", "reason": "<reason>"}`. `error`
  keeps the server's usual form; `reason` is the Source's own words, which
  the web shows. The server names no format.

*Alternative:* put only the reason in `error`. Every other refusal's `error`
starts with what went wrong, and a client that logs `error` would lose that.

### 10. The web shows the preview and the server's refusal

- `toEntry` accepts `preview_url` only when it is `null` or exactly
  `/api/assets/<id>/preview`, and gives `Entry.imageUrl`, the preview or else
  the original. `AssetPage.tsx`'s `assetOf` does the same. `Library.tsx`
  and `AssetPage.tsx` use `imageUrl` in their `<img>`.
- `AssetPage.tsx`'s map from media type to name gains `image/tiff` as
  `TIFF` (D16).
- **The refusal line (D14).** For a 415, `upload.ts` reads the body's
  `reason`. When it is text of 1 to 200 characters with no control
  character, the line's reason is that text, followed by a full stop unless
  it ends with one. Otherwise the line says `its format is not supported.`
  The reason is shown as React text, so markup never runs. The 400 text
  becomes `choose one image file and try again.` The comment that says the
  server's words never reach the screen changes to say that only a 415's
  reason does.
- **No list of formats.** The picker has no `accept` attribute, so it offers
  any file, and the server refuses what the pack does not take. The empty
  library reads `No images yet. Upload an image to start.`
- **Tokens:** none. The screens' layout and styles do not change.

*Alternatives:* `accept="image/*"`. It still guesses the pack's list, and
it would hide a FITS file, whose media type is not `image/*`, once
`packs/astro` takes one. A server endpoint that lists the accepted media
types. It is more API than one picker needs today.

## Risks / Trade-offs

- [A codec uses memory before a caller limit applies or during pixel decode]
  → the decoder performs all file-dependent allocations inside its fixed
  arena, and the helper's exit cannot
  publish a partial preview or interrupt the server.
- [Copying input to a seekable temporary file costs I/O and disk space]
  → copy in fixed chunks, keep at most one copy per running preview and
  close it on success, refusal or cancellation.
- [Several uploads at once each decode a large image] → decision 9 allows one
  preview run at a time per server.
- [A large TIFF makes the upload's answer late (D7)] → the web keeps the
  file's line at `Uploading` until the answer comes. The helper has a
  deadline so later previews can proceed.
- [Every new upload stores a second object (D1)] → accepted. A JPEG at
  quality 90 is usually much smaller than a 16-bit TIFF.
- [A JPEG original loses a little quality in its preview] → accepted:
  quality 90 with 4:4:4 sampling.
- [A wide-gamut or tagged TIFF (Adobe RGB, ProPhoto) looks less saturated,
  because the preview carries no colour profile] → out of scope. A follow-up
  can copy the ICC profile into the JPEG; `jpeg-encoder` can write one.
- [A phone JPEG with an EXIF rotation shows unrotated, where the browser
  rotated the original before] → the spec says so. Astro images rarely carry
  a rotation; a follow-up can apply it.
- [A panic in a decoder] → `spawn_blocking` already turns a panic into a
  failed run, `the operator stopped`.
- [The contract changes for third-party packs (ABI 3), and every Operator
  must now list its media types] → the contract is unfrozen until M2. The
  missing method is a compile error that names it, and `makes_previews` has
  a default.
- [An Operator with a wrong list stops running, and a run core does not
  accept leaves no trace] → registration refuses an empty or malformed list,
  and the image pack's lists come from the same table as recognition. The
  upload tests assert that each upload Operator has a run for a PNG, a JPEG
  and a TIFF.
- [An Operator trusts the recorded media type, so a stored object replaced
  by other bytes is read as the wrong format] → it fails as a header or
  image of the recorded format, with a clear reason. Finding a replaced
  object is left to a later verify-and-repair feature.
- [A 415 reason from a pack could be long or odd] → the web shows it only as
  text, at most 200 characters with no control character, and otherwise
  shows its own words.
- [The picker offers any file] → the server refuses what the pack does not
  take, and the file's line shows the pack's reason.

## Migration & Rollback

- **Forward:** migration 6 adds `assets.origin`, filled with `source` for
  every existing row, and the empty `asset_lineage` table. Generate
  `core/schema/6.json` with `mise run db-schema`, add it to `baselines()`, and
  move the startup tests and `scripts/instance-verify.sh` from five to six
  migrations. Update raw fixture inserts that use the latest schema to name
  `origin`. Fixtures that intentionally use migration 5 keep their old
  inserts. No previews are backfilled (D8).
- **A database that ran #547's earlier build:** #547's first build had a
  different migration 6, which #547 removed. A database that applied it
  holds six records whose sixth does not match this change's migration 6,
  and the server refuses it at startup with `incompatible migration
  history`. Use a fresh database, as #547's design says.
- **Rollback:** migrations are forward-only (ADR-004), and a server built
  before this change refuses a database with six migrations. To roll back,
  restore the database backup taken before the upgrade. Preview objects left
  in storage are then unreferenced and harmless.
