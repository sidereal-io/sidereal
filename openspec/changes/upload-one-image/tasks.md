## 1. Shared contracts

Run commands from the repository root through the configured mise environment.
Database tests require the fixture started with `mise run db-up` and
`TEST_DATABASE_URL` set from `mise run db-test-url`. Test filters below name
the behavior-focused tests to add; run each failing test before its implementation.

- [ ] 1.1 [medium] Persist an Asset with a same-Asset current-version reference and immutable version data through migration 4; cover fresh startup, upgrade from version 3 and invalid references, then regenerate the normalized snapshot with `mise run db-schema` (verify: cargo test --manifest-path server/Cargo.toml -p sidereal-core asset_schema)
- [ ] 1.2 [medium] Execute a Registry-selected Source through a bounded candidate-reading and single-proposal AssetContext, with host-owned identity/provenance and explicit refusal; adapt the astro stub and ABI version as design decision 1 describes (verify: cargo test --manifest-path server/Cargo.toml -p sidereal-plugin-abi source_execution)
- [ ] 1.3 [medium] Register the image pack's manual-upload Source with byte recognition for PNG/JPEG, including misleading filenames, unsupported bytes and recognizable damaged images; enforce that all packs depend on neither core/server nor each other through the extended architecture lint (verify: cargo test --manifest-path server/Cargo.toml -p sidereal-pack-image)

## 2. Managed ingest [depends-on: 1]

- [ ] 2.1 [complex] Stage an upload through bounded writes into an exclusively created temporary file under the paired root, refusing the 100 MiB plus one-byte boundary and unsafe directory/file entries without using the display name as a path (verify: cargo test --manifest-path server/Cargo.toml -p sidereal-core ingest_staging)
- [ ] 2.2 [complex] Publish one complete BLAKE3-addressed object with file and ancestor-directory sync, reusing an existing object without replacement even during concurrent uploads; test no-follow access, non-regular files and failed publication as design decision 3 describes (verify: cargo test --manifest-path server/Cargo.toml -p sidereal-core object_publication)
- [ ] 2.3 [complex] Commit the initial Asset/version pair only after registered Source acceptance and durable object publication; inject Source, write, sync and database failures to prove no partial records and no deletion of shared or published objects (verify: cargo test --manifest-path server/Cargo.toml -p sidereal-core ingest_commit)

## 3. Guarded HTTP [depends-on: 2]

- [ ] 3.1 [complex] Accept one multipart file through POST /api/assets using the production Registry and storage service; test success, exact-size and streamed over-limit inputs, extra fields, malformed trailing data, format refusal, missing storage, and existing session/origin refusals with no committed Asset on rejection (verify: cargo test --manifest-path server/Cargo.toml -p sidereal-server --test assets upload)
- [ ] 3.2 [medium] List committed Asset entries through GET /api/assets, covering empty, populated, persistence failure and invalid-session responses without listing temporary or unreferenced objects (verify: cargo test --manifest-path server/Cargo.toml -p sidereal-server --test assets list)
- [ ] 3.3 [medium] Stream original bytes through GET /api/assets/{id}/original with the recognized Content-Type and private response headers, covering unknown ids, unavailable objects, symlink refusal and absent/expired/revoked sessions (verify: cargo test --manifest-path server/Cargo.toml -p sidereal-server --test assets original)

## 4. Signed-in library [depends-on: 3]

Before building the visual components in this section, follow `visual-options`
and DESIGN.md's component-choice process. Record the selected treatment in
DESIGN.md using existing tokens. The plan adds no tokens; a choice needing new
values must return to planning. Keep the existing health and access tests green.

- [ ] 4.1 [medium] Display committed images and literal filenames in a token-styled Library within Access's signed-in branch, with loading/empty/list-error/retry and image-display-error states, removing the library on sign-out or session loss and ignoring late responses (verify: pnpm --dir web test -- Library.test.tsx Access.test.tsx)
- [ ] 4.2 [medium] Let Nova select and upload one image, showing the filename and pending state, preventing a second submission, displaying fixed refusal messages and refreshing the library after success without a page reload (verify: pnpm --dir web test -- Library.test.tsx)

## 5. Integration [depends-on: 4]

- [ ] 5.1 [complex] Demonstrate the complete first-image journey with a real PNG and JPEG: a Rust process test retains ids, provenance and identical original bytes across restart, and an isolated browser test renders the uploaded image; run the browser scenario with `mise run e2e`, then finish the repository gate (verify: mise run check)

## Deviations Ledger

None.
