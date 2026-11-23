<!-- Plain language: Each task must make sense to an agent and reviewer without the planning conversation. Name an observable outcome, its location when needed, and how to verify it. Preserve checkbox syntax and tags. -->

<!-- Run cargo, pnpm, `mise run e2e` and `mise run check` through `deli -- <command>` on this machine, with long timeouts. -->

## 1. Core finds the existing Asset

- [x] 1.1 [medium] In `server/crates/core`, make `ingest` return `Ingested::Created(AssetRecord)` or `Ingested::Found(AssetRecord)` (design decision 4). In the Asset transaction, look for a version with the candidate's hash before inserting (design decisions 1 and 3). A second upload of the same bytes as `m31-final.png` returns `Found` with the first Asset, unchanged and still named `nebula.png`, and the database still holds one Asset and one version; different bytes return `Created`. Rewrite `a_failed_commit_after_reuse_keeps_the_shared_object` as the design's Risks section says: the first commit fails and keeps the object, the test drops the failure trigger, then a second upload of the same bytes reuses that object and returns `Created`. Update the route's call so the crate builds, still answering 201 for both results until task 2.1 (verify: cargo test --manifest-path server/Cargo.toml -p sidereal-core ingest_commit) (✓ cargo test -p sidereal-core ingest_commit passed: 18 tests)
- [x] 1.2 [medium] An upload finds the oldest matching Asset: with two Assets whose versions share a hash (the test inserts the second one directly), ingest returns the Asset whose version has the earlier `ingested_at`, then the lower Asset id, and both Assets and their versions stay unchanged. It also finds an Asset whose matching version is not its current version (the test inserts that version directly). With a directly inserted version that has the upload's hash and another Source's `source_id`, an accepted upload returns `Found` and that version keeps its provenance; with a directly inserted version that has the hash of `r…` bytes, uploading those bytes returns `Refused` and changes no rows (verify: cargo test --manifest-path server/Cargo.toml -p sidereal-core ingest_commit) (✓ cargo test -p sidereal-core ingest_commit passed: 23 tests; mutation checks in the Deviations Ledger)
- [x] 1.3 [complex] Concurrent uploads of the same new bytes make one Asset: start the Asset transaction with `SET TRANSACTION ISOLATION LEVEL READ COMMITTED`, then take `pg_advisory_xact_lock` on a new "Asset content" key and the key from the hash's first 8 hex digits (parsed as `u32`, kept as `i32` bits) in its own statement before the lookup (design decision 1). A unit test pins the key for `00000000…`, `7fffffff…`, `80000000…` (`i32::MIN`) and `ffffffff…` (`-1`). The concurrency test holds upload A before commit with a held advisory lock and an `AFTER INSERT` trigger on `public.asset_versions`, as the design's Risks section says, then starts upload B and waits until two backends wait on advisory locks, with bounded waits. After the release it asserts exactly one `Created`, one `Found` for the same Asset, and one Asset and one version for the hash. It runs once with the default isolation and once with `default_transaction_isolation` set to `repeatable read` on the Database's connections (verify: cargo test --manifest-path server/Cargo.toml -p sidereal-core ingest_commit) (✓ cargo test -p sidereal-core ingest_commit passed: 26 tests; with the lock alone, only the repeatable-read test failed)

## 2. The upload route answers 409 [depends-on: 1]

- [x] 2.1 [parallel] [medium] `POST /api/assets` answers 409 with `{ "asset": entry }` for `Ingested::Found` and 201 for `Created`. In `server/crates/server/tests/assets.rs`, upload `nebula.png`, then the same bytes as `nebula.png` and as `m31-final.png`: the first answer is 201, each repeat is 409 with the first answer's `id`, `kind`, `current_version_id` and `original_url` and `name` `nebula.png`, and `GET /api/assets` still lists one Asset named `nebula.png`. Plain text named `nebula.png` still answers 415. The tests `upload_onto_an_object_whose_bytes_changed_answers_503` and `upload_onto_a_much_larger_object_answers_503_and_leaves_it_unchanged` still pass unchanged, so a repeat upload onto a damaged object answers 503, not 409 (verify: cargo test --manifest-path server/Cargo.toml -p sidereal-server --test assets upload) (✓ cargo test -p sidereal-server --test assets upload passed: 26 tests, both 503 tests unchanged)

## 3. The library screen

- [x] 3.1 [medium] In `web/src/Library.tsx`, a 409 answer shows `<chosen name> is already in the library.`, or `<chosen name> is already in the library as <existing name>.` when the body's checked `asset` entry has another name (design decision 5). The body is checked with the same rules as a list entry, and a body with no valid entry gives the shorter notice with no name from the body. The screen shows no `was not uploaded` message, does not request the list again, keeps the Upload button enabled and clears the picker so another file can be chosen at once; names appear as text only (verify: pnpm --dir web test -- Library.test.tsx) (✓ pnpm --dir web test -- Library.test.tsx passed: 194 tests)
- [x] 3.2 [easy] When an upload gets no answer, the screen shows `No answer came back for <chosen name>. It is safe to upload it again: if it was saved, Sidereal will find it.`, loads the list again and sends exactly one upload request. Update the existing test "says the outcome is unknown, and loads the list again, when no answer comes back" (verify: pnpm --dir web test -- Library.test.tsx) (✓ pnpm --dir web test -- Library.test.tsx passed: 194 tests)

## 4. Journey and gate [depends-on: 2, 3]

- [x] 4.1 [medium] In `web/e2e/upload.spec.ts`, after Nova uploads `nebula.png` and `nebula.jpg`, Nova uploads `nebula.png`'s bytes again as `m31-final.png` (Playwright `setFiles` with a name and a buffer). The screen shows `m31-final.png is already in the library as nebula.png.`, and the library still shows exactly two images, before and after the restart (verify: mise run e2e) (✓ mise run e2e passed: 2 tests; mutation check in the Deviations Ledger)
- [x] 4.2 [easy] The whole gate passes: format, lint, unit and integration tests, the architecture lint, strict OpenSpec validation and the web checks (verify: mise run check) (✓ mise run check passed: compose, openspec 14/14, server, arch lint, web 194 tests)

## Deviations Ledger

<!-- Dated notes on small departures from the plan, written during apply. Stop and ask the user about a large one. -->

- 2026-11-23, task 1.2: its tests passed on their first run, because task 1.1
  already wrote design decision 3's lookup query (any version, oldest
  `ingested_at`, then lowest Asset id). To show the tests catch a wrong
  lookup, the query was broken on purpose and then restored. Ordering by
  newest first, with current versions only, failed the oldest, lowest-id and
  not-current tests. Matching only the uploading Source's `source_id` failed
  the provenance test.
- 2026-11-23, task 4.1: the journey step passed on its first run, because
  tasks 2.1 and 3.1 had already built the 409 answer and the notice. To show
  the step catches a regression, the route was made to answer 201 for a found
  Asset and then restored: the journey then failed, waiting for the notice
  `m31-final.png is already in the library as nebula.png.`
