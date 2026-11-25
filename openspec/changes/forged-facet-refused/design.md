## Context

The behavior this change pins down already exists. It has no test at the level that matters:

- `AssetContext::propose_facet` in `server/crates/plugin-abi/src/lib.rs` takes a facet name and a value, and nothing else. `store` in `server/crates/core/src/operations.rs` picks the subject from the facet's scope and the run's own version row: `version.asset_id` for an Asset facet, the run's `asset_version_id` for an Asset version facet. No test runs this while other Assets or versions exist, so a mistake that writes against the wrong id could pass. `an_asset_value_belongs_to_the_asset_that_owns_the_version` in `stores.rs` uses one Asset only.
- `AssetContext::propose_asset` refuses an Operator run with `ContextError::NotAllowed` and sets the run's broken reason to `an operator run proposed an asset`. `Registry::execute_operator` then returns `ExecutionError::Failed` with that reason. The test `operator_execution_refuses_an_asset_proposal` covers this inside `plugin-abi`. No core test shows that the operation run row is `failed`, and that no Asset, version or facet value is added.
- The core test fixture, `server/crates/core/src/operations/fixture.rs`, makes a new Asset with one version per call of `Fixture::version`. Ingest has no way yet to add a second version to an existing Asset.

## Goals / Non-Goals

**Goals:**

- Two core tests, one per new requirement in `specs/asset-facets/spec.md`.
- Each test fails if the behavior it guards breaks. The tasks prove this with a temporary break that is not committed.

**Non-Goals:**

- No change to `plugin-abi`, to core's production code, or to the database schema.
- No tests through the HTTP upload path. Core's run is the only place that stores facet values. A server-level test would reach the same code through more setup.
- No recheck of the input after a run (D4 in `brainstorm.md`).

## Test Seams

- **Primary test seam:** `run_operator` in `server/crates/core`, driven through the existing `Fixture` against a real PostgreSQL test database.
- **Existing seams reused:** `fixture.rs` (`Fixture`, `op`, `count`, `values`, `runs`, `failed_reason`), `stores.rs` for the stays-on-its-input test, `refuses.rs` for the asset-proposal test.

## Scrutiny Tier & Blast Radius

- **Tier:** low
- **Blast radius:** test code in `server/crates/core/src/operations/` only. The new requirements describe the plugin trust boundary, but this change adds no code to it.

## Decisions

1. **Make a second version of the same Asset with a direct SQL insert.** Add a fixture method, such as `Fixture::another_version(&AssetRecord)`, that inserts one `asset_versions` row for the same `asset_id` with a fresh id and a made-up hash. It does not change `current_version_id`. The run never reads this version, so it needs no stored object. Alternative: add a second-version path to ingest. Rejected: that is product behavior no story has asked for yet.
2. **Check the subject of each stored value, not only the count.** The stays-on-its-input test reads every row of `facet_values` and asserts each `asset_id` and `asset_version_id` exactly. Counting values per scope would miss a value written against the wrong subject of the right scope, such as A2 instead of A1. The database already refuses a value with both ids set or neither.
3. **Count Assets and versions before and after the refused run.** The asset-proposal test compares `count(*)` of `assets` and `asset_versions` before and after. Alternative: check that no row has the proposed kind. Rejected: the fixture's own Source already makes `image` Assets, so a count is the clear measure.
4. **Match the reason by its words.** The test asserts that the reason contains `proposed an asset`. The full text comes from `plugin-abi`, and the spec names only its meaning.
5. **No ADR.** Nothing here is costly to reverse: these are tests of behavior earlier changes already decided.

## Risks / Trade-offs

- [A test written after the code passes from the start, so it may prove nothing] → Each task breaks the guarded code on purpose, sees the test fail, and restores it before committing. Task 1.2 breaks `store`'s choice of subject. Task 2.2 removes the broken-reason check in `execute_operator`.
- [#526 will let an Operator propose an Asset, as a derived asset] → #526 must change the requirement "An Operator run makes no Asset" and its test. The proposal's Impact section says so, so #526's plan finds it.
- [A made-up hash in the extra version could clash with a real one] → Use a hash no fixture bytes produce, such as 64 `f` characters. No stored object needs to match it.

## Migration & Rollback

N/A. Tests only.
