## 1. Shared contracts

Run commands from the repository root through the configured mise environment.
Run heavy commands through `deli --`. Database tests require the fixture
started with `mise run db-up` and `TEST_DATABASE_URL` set from
`mise run db-test-url`. Test filters below name the behavior-focused tests to
add; run each failing test before its implementation.

- [ ] 1.1 [medium] Persist an Asset with a same-Asset current-version reference and immutable version data through migration 4; cover fresh startup, upgrade from version 3 with a claimed user unchanged, concurrent startup with exactly four records, and invalid references; update the existing table-list and record-count assertions in `core/src/database/startup/tests.rs` and `server/tests/siderealctl.rs` as the `postgres-foundation` and `security-events` deltas say; then regenerate the normalized snapshot with `mise run db-schema` (verify: cargo test --manifest-path server/Cargo.toml -p sidereal-core asset_schema, then cargo test --manifest-path server/Cargo.toml -p sidereal-core startup, then cargo test --manifest-path server/Cargo.toml -p sidereal-server --test siderealctl)
- [ ] 1.2 [medium] Execute a Registry-selected Source through a bounded candidate-reading and single-proposal AssetContext, with host-owned identity/provenance and explicit refusal; adapt the astro stub and ABI version as design decision 1 describes (verify: cargo test --manifest-path server/Cargo.toml -p sidereal-plugin-abi source_execution)
- [ ] 1.3 [medium] Register the image pack's manual-upload Source with byte recognition for PNG/JPEG, including misleading filenames, unsupported bytes and recognizable damaged images; extend the architecture lint so no pack depends on core, server or another pack, and add a failure fixture that the lint rejects (verify: cargo test --manifest-path server/Cargo.toml -p sidereal-pack-image, then sh server/scripts/check-arch.sh, which must also show the fixture refused)

## 2. Managed ingest [depends-on: 1]

- [ ] 2.1 [complex] Open the paired root once, check its marker UUID through that open folder, and stage an upload through bounded writes into an exclusively created temporary file under it; assert the root holds only its marker after startup and gains `tmp` and `store` only at first ingest; refuse a root swapped between pairing and object-store start; refuse the 100 MiB plus one-byte boundary and unsafe directory/file entries without using the display name as a path (verify: cargo test --manifest-path server/Cargo.toml -p sidereal-core ingest_staging)
- [ ] 2.2 [complex] Publish one complete BLAKE3-addressed object, syncing the file and the full directory chain on every upload; test two hashes under a fresh prefix with the first publisher paused before its directory sync; re-hash an existing object before reuse and refuse a same-size object with changed bytes with a storage failure that leaves it unchanged; reuse a matching object without replacement during concurrent uploads; test no-follow access, non-regular files and failed publication as design decision 3 describes (verify: cargo test --manifest-path server/Cargo.toml -p sidereal-core object_publication)
- [ ] 2.3 [complex] Commit the initial Asset/version pair only after registered Source acceptance and durable object publication; with a test Source registered through the public contract, assert the persisted provenance names that Source; inject Source, write, sync and database failures to prove no partial records and no deletion of shared or published objects (verify: cargo test --manifest-path server/Cargo.toml -p sidereal-core ingest_commit)

## 3. Guarded HTTP [depends-on: 2]

- [ ] 3.1 [complex] Accept one multipart file through POST /api/assets using the production Registry and storage service; test success, exact-size and streamed over-limit inputs, extra fields, malformed trailing data, format refusal, a hash-mismatched existing object (503), and missing storage with health and readiness unchanged; on 401 and 403 assert no Asset is committed and the object store's file list is unchanged (verify: cargo test --manifest-path server/Cargo.toml -p sidereal-server --test assets upload)
- [ ] 3.2 [medium] List committed Asset entries through GET /api/assets, covering empty, populated, persistence failure and invalid-session responses without listing temporary or unreferenced objects (verify: cargo test --manifest-path server/Cargo.toml -p sidereal-server --test assets list)
- [ ] 3.3 [medium] Stream original bytes through GET /api/assets/{id}/original with the recognized Content-Type and private response headers, covering unknown ids, unavailable objects, symlink refusal and absent/expired/revoked sessions (verify: cargo test --manifest-path server/Cargo.toml -p sidereal-server --test assets original)
- [ ] 3.4 [medium] Refuse to start when `STORAGE_ROOT` and `WEB_ROOT` are the same folder or one holds the other, after following symbolic links, with a message naming both, before any database write; let separate folders start; add an unauthenticated request for a stored object's path through the web-file fallback that returns no bytes; say in `docs/install.md` that the two folders must not overlap (verify: cargo test --manifest-path server/Cargo.toml -p sidereal-server web_root_overlap, then cargo test --manifest-path server/Cargo.toml -p sidereal-server --test assets web_fallback)

## 4. Signed-in library [depends-on: 3]

Before building the visual components in this section, follow `visual-options`
and DESIGN.md's component-choice process. Record the selected treatment in
DESIGN.md using the tokens from task 4.1. A choice needing token values beyond
design decision 5's table must return to planning. Keep the existing health and
access tests green.

- [ ] 4.1 [medium] Add the six semantic color tokens of design decision 5 to DESIGN.md first; teach `web/scripts/tokens.ts` to write a semantic token as `var()` of the primitive it references, still refusing raw values, unknown references and component tokens; run `pnpm tokens` in `web/`; move `Access.css` and `styles/base.css` to the semantic tokens for those roles, keeping the submit button's computed color (verify: pnpm --dir web test -- tokens.test.ts tokens-in-use.test.ts Access.test.tsx, then mise run check-web)
- [ ] 4.2 [medium] Display committed images and literal filenames in a token-styled Library within Access's signed-in branch, with loading/empty/list-error/retry and image-display-error states, removing the library on sign-out or session loss and ignoring late responses (verify: pnpm --dir web test -- Library.test.tsx Access.test.tsx)
- [ ] 4.3 [medium] Let Nova select and upload one image, showing the filename and pending state, preventing a second submission, displaying fixed refusal messages and refreshing the library after success without a page reload (verify: pnpm --dir web test -- Library.test.tsx)

## 5. Integration [depends-on: 4]

- [ ] 5.1 [complex] Make `mise run e2e` run on its own throwaway stack as the design's Test Seams section describes: a `sidereal_test_e2e_` database made through the test role, a temporary `STORAGE_ROOT`, a server and Vite on checkout-derived ports, no reuse of a running stack, an ignored inherited `DATABASE_URL`, cleanup on pass and fail, and refusal before any browser test when setup fails; make `vite.config.ts` honor `SIDEREAL_DEV_PROXY_TARGET`; remove the CI `e2e` job's database URL step; check the `dev-commands` scenarios by running them with `mise run dev` up, with a failing test, with an inherited `DATABASE_URL` and with the fixture down (verify: mise run e2e, then mise run check-compose, then the ci delta's `yq` commands)
- [ ] 5.2 [complex] Demonstrate the complete first-image journey with a real PNG and JPEG: a Rust process test retains ids, provenance and identical original bytes across restart; the isolated browser test claims, signs in, uploads, sees the rendered image, restarts the server it started, reloads, and still sees the name and image; then finish the repository gate (verify: mise run e2e, then mise run check)

## Deviations Ledger

None.
