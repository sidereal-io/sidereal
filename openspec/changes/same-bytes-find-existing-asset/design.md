<!-- Plain language: Help the next implementer find the approach, understand the choices and act on the plan. Explain necessary technical terms. Put each risk beside its mitigation and each open question beside the decision it affects. -->

## Context

#521 built one upload path, and this change adds to it:

1. The route `upload` in `server/crates/server/src/assets.rs` streams the file
   into staging. Staging computes the BLAKE3 hash as the bytes arrive.
2. `ingest` in `server/crates/core/src/ingest.rs` runs the manual-upload
   Source, then `ObjectStore::publish`. When an object already exists at the
   hash path, `publish` re-hashes it and reuses it (`Published.reused`).
3. `Database::create_asset` in `server/crates/core/src/assets.rs` inserts the
   Asset and its first version in one transaction.

Nothing looks for an existing version, so the same bytes make a second Asset
that shares the stored object. The schema already has what a lookup needs:
`asset_versions.content_hash` has a plain index (`asset_versions_content_hash`,
migration 4). ADR-003 and ADR-011 keep the hash **indexed but not unique**,
because later versions and derived assets may share bytes.

The web side is `web/src/Library.tsx`. `sendFile` maps an answer to `saved`,
`unconfirmed` or `{ refused }`, and the component shows one status line under
the bar for each state.

## Goals / Non-Goals

**Goals:**

- Find the existing Asset inside the same transaction that would create a new
  one, so the check and the insert cannot be split by another upload.
- Keep the API's entry shape: the 409 body reuses the `entry` function.
- Keep the first-upload path, its ADR-011 order and its failure answers as
  they are.

**Non-Goals:**

- Skipping the Source run or the publication for a repeat upload. Both still
  run, as described in Decision 2.
- Any schema migration, unique constraint or change to the stored objects.
- Opening the found image, merging Assets, or near-duplicate matching.

## Test Seams

- **Primary test seam:** the server's in-process route tests in
  `server/crates/server/tests/assets.rs`. They drive `POST /api/assets` and
  `GET /api/assets` against a fresh database and paired storage, and they
  prove the 409 answer, its body, the unchanged name and the single row.
- **Existing seams reused:**
  - `server/crates/core/src/ingest/tests.rs` (its `Fixture`, with storage
    hooks). It holds the concurrent-upload test, the oldest-of-two lookup
    (it can insert a second Asset with the same hash directly), and the
    rewritten reuse-after-failed-commit test.
  - `web/src/Library.test.tsx` (`stubServer`, `choose`, `uploads`) for the
    notice and the lost-answer message.
  - `web/e2e/upload.spec.ts` gains one step: Nova uploads `nebula.png`'s
    bytes again as `m31-final.png` and sees the notice.

## Scrutiny Tier & Blast Radius

- **Tier:** medium. One subsystem on established patterns, with no migration
  and no new contract in `plugin-abi`. It changes one public API answer
  (201 to 409 for a repeat upload) and adds a lock in the ingest commit.
- **Blast radius:** `core::ingest` and `core::assets` (the Asset transaction),
  the upload route's answer, and the library screen's upload states. Listing,
  original retrieval and storage are untouched.

## Decisions

1. **The lookup runs inside the Asset transaction, under a lock per hash.**
   `create_asset` becomes a find-or-create step. It opens the transaction,
   takes `pg_advisory_xact_lock(<key1>, <key2>)`, then looks for a version
   with the hash. If it finds one, it returns that Asset and inserts nothing.
   Otherwise it inserts as today and commits. `<key1>` is a new constant for
   "Asset content", distinct from the startup lock's `1397310533`; `<key2>` is
   the first 8 hex digits of the hash as an `i32`. The lock is released at
   commit, and under READ COMMITTED a query that starts after the lock is
   granted sees the other upload's committed rows. Two hashes with the same
   first 8 digits only wait for each other; they stay correct.
   - *Instead: a unique index on `content_hash`.* Rejected: ADR-003 and
     ADR-011 keep the hash non-unique, and #526's derived assets may share
     bytes.
   - *Instead: SERIALIZABLE transactions with a retry.* Rejected: it needs a
     retry loop and gives no clearer result for one hash.
   - *Instead: a lookup before publication only.* Rejected as the only check:
     two concurrent uploads would both find nothing. It is not added as an
     early shortcut either; one check is simpler to test.
   - *Instead: a lock in the server process.* Rejected: it does not hold
     across two server processes on one database.

2. **A repeat upload still runs the Source and publication.** The Source must
   accept the bytes before ingest trusts the hash, and `publish` already
   reuses and re-checks an existing object. Doing both keeps one ingest order
   for every upload. The cost is one re-hash of the stored object per repeat
   upload, at most 100 MiB.

3. **Which Asset is found.** The lookup is
   `SELECT v.asset_id FROM public.asset_versions v WHERE v.content_hash = $1
   ORDER BY v.ingested_at, v.asset_id LIMIT 1`, then the existing `select()`
   for that Asset, so the result has its current version. It matches any
   version, not only current ones (D1), and orders as D2 says.

4. **`ingest` returns which way it went.** It returns
   `Result<Ingested, IngestError>`, where
   `enum Ingested { Created(AssetRecord), Found(AssetRecord) }`. The route
   maps `Created` to 201 and `Found` to 409, each with
   `{ "asset": entry(&record) }`. `IngestError` and its failure answers do not
   change. A Found result is not a failure, so the route logs nothing for it.
   The filename is used only for a new Asset (D3).

5. **The web client checks the 409 body like a list entry.** `entries`
   gets a single-entry helper that both the list and the 409 answer use, with
   the same checks (`original_url` must be the Asset's own route). `sendFile`
   gains the result `{ found: string | null }`, holding the existing name or
   `null` when the body has no valid entry. The component gets an
   `Upload` state `{ kind: "found", name, existing }` and shows the notice in
   the same `library-status` line as the other states. A 409 answer does not
   increment `generation`, so the list is not requested again. Only the chosen
   name and the checked existing name reach the screen, both as text.

6. **The lost-answer message changes text only.** The `unconfirmed` state
   keeps its behavior: one request, no retry, and the list loads again.

Tokens added: none. The notice uses the existing `library-status` style, as
every other upload state does.

No decision here is costly to reverse: none adds a schema, a contract or a
stored format.

## Risks / Trade-offs

- [A concurrency test passes by luck when the two uploads do not overlap] →
  The core test holds both ingests at a `std::sync::Barrier` in the storage
  hook on the last directory sync, so both reach the Asset transaction
  together. It runs several rounds with fresh bytes, and asserts exactly one
  `Created` and one row pair each round.
- [The existing test `a_failed_commit_after_reuse_keeps_the_shared_object`
  no longer reaches the insert, because the second upload finds the first
  Asset] → Rewrite it: the first upload's commit fails and leaves the object;
  a second upload of the same bytes then reuses that object and creates the
  Asset.
- [A client that treated every non-201 answer as a failure now shows a 409
  as one] → The only client is `web/`, which this change updates. No outside
  client exists yet.
- [A repeat upload of an Asset whose stored object is damaged answers 503,
  because `publish` refuses to reuse it] → This is the current rule for
  reused objects. Checking and repair belong to #371.
- [The lock adds a wait for uploads of the same bytes, or of hashes that share
  the first 8 hex digits] → The wait covers one short transaction. Unrelated
  hashes almost never share a key.

## Migration & Rollback

N/A. Nothing changes shape: no migration, no stored format and no
configuration. Rolling back the code brings back 201 and a second Asset for a
repeat upload. Assets made by either version stay valid.
