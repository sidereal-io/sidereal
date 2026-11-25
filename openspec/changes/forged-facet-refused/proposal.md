## Why

Pat writes plugins against a contract that will change until M2. Nothing yet proves that an Operator can write only to its own input, or that an Operator run cannot make an Asset. A later change could loosen either rule without a test failing. #524 already tests the other refusals: undeclared facets and values that fail their schema (D1).

## What Changes

- The `asset-facets` spec states that the Asset context gives an Operator no way to name an Asset or Asset version. An Operator's values reach only the version its run read, or the Asset that owns that version (D3).
- A new core test runs an Operator over one Asset version while another Asset, and another version of the same Asset, exist. It checks that the run's values land only on its own input (D2, D3).
- The `asset-facets` spec states that core records an Operator run that proposes an Asset as failed, and adds no Asset, version or facet value. A new core test proves it end to end. Today only `plugin-abi`'s own unit test covers this (D2).
- No new refusal logic, and no change to `plugin-abi`'s public API. The tests fix behavior that already exists.
- Out of scope: core rechecking an Operator's input after a run. It goes to a new issue under epic #356 (D4).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `asset-facets`: the Operator contract names no target; values stay on the run's own input while other Assets and versions exist; an Operator run that proposes an Asset is recorded failed and adds nothing.

## Impact

- Tests only, in `server/crates/core/src/operations/` (`stores.rs`, `refuses.rs`, `fixture.rs`). They run in `cargo test`, which `mise run check` runs.
- No change to `plugin-abi`, the database schema, the HTTP API or `web/`.
- #526 (TIFF preview as a derived asset) will let an Operator propose an Asset. It must then change the new "proposes an Asset" requirement and its test.
