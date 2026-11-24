## Why

Each worktree's `server/target/` grows to several GiB, and a machine that runs many agents in parallel, each in its own worktree, runs out of disk. One fresh build under Cargo's default profile takes 3.1 GiB, mostly debug information, and Cargo never deletes old output, so a worktree keeps growing after that.

## What Changes

- Add a lighter development build profile to `server/Cargo.toml`: line tables only for our own crates, and no debug information for dependencies (D1). It applies to every contributor and to CI, and covers the debug build, `cargo test` and `cargo clippy --all-targets`. The release build keeps Cargo's default, which already has no debug information. One fresh build measured 1.3 GiB with these settings.
- Add a `target-clean` development task. It deletes this worktree's `server/target/` only when the folder is over a size limit, which a contributor can change. It runs as `mise run target-clean`, or as one plain command without mise. `mise run check` does not run it (D3).
- Document in `CONTRIBUTING.md`:
  - why the profile is lighter, and how to get full debug information back for a debugging session (D1);
  - the clean-up task, its size limit and its plain command (D3);
  - one build folder shared by all worktrees, only as a personal option, with its trade-off (D4);
  - sccache, only as a personal option, with the decision the measurements support (D2).
- Record the measurements behind each choice, sccache included, in the design and the pull request (D2, D5). Nothing in the gate measures disk size (D5).

Nothing is removed and nothing breaks. Panic backtraces keep file and line numbers. A debugger shows fewer details until a contributor turns full debug information back on.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `dev-environment`: adds a requirement that development builds keep debug information light, checked by reading `server/Cargo.toml`.
- `dev-commands`: the development task list grows from fourteen to fifteen tasks with `target-clean`, and a new requirement says what the clean-up task deletes and when.

## Impact

- **Build configuration:** `server/Cargo.toml` gains `[profile.dev]` settings, which `cargo test` and `cargo clippy` use too. The first build after the change rebuilds everything, once, on every machine.
- **Tasks and scripts:** `mise.toml` gains the `target-clean` task, and `server/scripts/` gains the script it runs.
- **Documentation:** `CONTRIBUTING.md` (Development Commands, Development Environment and the task table for contributors without mise).
- **CI:** `Swatinem/rust-cache` keys its cache on the manifests, so the first run after the change misses the cache once. The workflow itself does not change (D7).
- **Routes kept working:** `mise run check`, `dev`, `e2e`, `instance-verify` and `external-db-verify`, and the hand-install route without mise (D6). The `Dockerfile` release build does not change.
- **Dependencies:** none added. sccache never becomes a requirement.
