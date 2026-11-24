## Context

- `server/Cargo.toml` has no `[profile.*]` tables and the repo has no `.cargo/config.toml`, so every build uses Cargo's defaults. The `dev` profile builds with full debug information (`-C debuginfo=2`) and incremental data for our own crates. The `test` profile inherits from `dev`. The `release` profile already has no debug information.
- Cargo never deletes outdated output by itself. After a dependency bump, a branch switch or a new stable Rust release, the old files stay next to the new ones. `cargo clean` in the current stable release deletes whole folders or packages; it has no stable option to delete by age or size.
- The plan step measured one worktree, on Linux x86-64 with the current stable Rust, all on the same machine. Each row starts from an empty `server/target/` and runs `cargo clippy --all-targets -- -D warnings`, then `cargo test --no-run`, in `server/`: the build half of `mise run check-server`.

  | Variant | `server/target/` | Own cache | Build time |
  |---|---|---|---|
  | Default profile (today) | 3.1 GiB | none | 57 s |
  | Lighter profile (D1) | 1.8 GiB | none | 53 s |
  | Lighter profile + sccache, cold cache | 1.8 GiB | 236 MiB | 60 s |
  | Lighter profile + sccache, warm cache, same path | 1.8 GiB | 236 MiB | 18 s |
  | Lighter profile + sccache, warm cache, another path | 1.8 GiB | 294 MiB | 40 s |

  Build times depend on the machine's load and are only a rough guide. The disk sizes repeat.
- Incremental data for our own crates is 718 MiB of the default build and 367 MiB of the lighter one. The lighter build has 10 files over 20 MiB.
- With sccache, the warm build at the same path served all 549 cacheable compiler calls from its cache, but the 118 calls it cannot cache still ran: linked binaries, proc-macros and incremental builds of our own crates.
- CI caches Cargo output with `Swatinem/rust-cache`, whose key includes a hash of every `Cargo.toml`. The `Dockerfile` builds `--release` in its own `/build/target`.
- `dev-commands` requires each development task to run one command or one POSIX `sh` script that works without mise, and `CONTRIBUTING.md` to list that command. `dev-environment` says mise is optional and never loads `.env`.

## Goals / Non-Goals

**Goals:**

- Cut what one build writes to each worktree's `server/target/`, for every contributor and for CI, without new tools.
- Give each worktree a size limit that a contributor or an outside scheduler can apply with one command.
- Keep every route and build configuration that D6 lists working.

**Non-Goals:**

- Changing the CI workflow or its cache (D7).
- Making sccache, a shared build folder or any other machine-specific setting a project setting (D2, D4).
- Deleting build output from inside `mise run check` or any other task that builds (D3).
- Cleaning other worktrees or Cargo's own caches in `~/.cargo`.

## Test Seams

- **Primary test seam:** a new POSIX `sh` check script, `server/scripts/target-clean-check.sh`. It builds each case of the `target-clean` spec in a scratch copy of the repo layout, under `mktemp -d`, runs the clean-up script there, and asserts the exit status, the output and what is left on disk. It needs no Rust toolchain and no Docker. It follows the pattern of `scripts/dev-supervisor-check.sh`, and like that script runs by hand: `sh server/scripts/target-clean-check.sh`.
- **Existing seams reused:** the profile is checked the way its spec scenarios say: by reading `server/Cargo.toml`, and by one verbose build of `sidereal-plugin-abi` (`cargo test --no-run -v -p sidereal-plugin-abi`), whose compiler command shows the `-C debuginfo` value. `mise run check` proves the gate still passes with the new profile, and `openspec validate` covers the spec deltas.

## Scrutiny Tier & Blast Radius

- **Tier:** low
- **Blast radius:** build configuration and contributor tooling only. `server/Cargo.toml` gains two profile tables, which change how every debug, test and clippy build compiles, but no crate's code or behavior. `mise.toml` gains one task, `server/scripts/` gains two scripts, and `CONTRIBUTING.md` gains documentation. The release build, the app image, the CI workflow and the server's runtime behavior do not change.

## Decisions

### 1. The lighter profile lives in `server/Cargo.toml` (D1)

```toml
[profile.dev]
debug = "line-tables-only"

[profile.dev.package."*"]
debug = false
```

`[profile.dev.package."*"]` applies to every package that is not a workspace member, so our four crates keep line tables and their dependencies get none. The `test` profile inherits from `dev`, so `cargo test` and `cargo clippy --all-targets` get the same settings.

Line tables keep file names and line numbers in panic backtraces and in profilers. A debugger can still step by line, but shows no local variables and no types.

Alternatives:

- **`debug = false` everywhere.** Smaller again, but panic backtraces in our crates lose their file and line numbers, which agents and people use to find a failure.
- **A committed `server/.cargo/config.toml`.** Cargo would apply it too, but a contributor's own `--config` and environment settings sit at the same level, which makes the override in decision 2 harder to explain. The manifest is where Cargo users look for profiles.
- **Turning incremental compilation off.** It would save the incremental share, 367 MiB with the lighter profile, but every edit-and-rebuild of our own crates would get slower. Incremental data is also replaced, not accumulated, as our crates change.
- **`split-debuginfo`.** On Linux it moves debug information into separate files next to the binaries; it does not make them smaller.

### 2. Full debug information comes back with a command-line override (D1)

`CONTRIBUTING.md` gives one command line for a debugging session:

```sh
cargo --config 'profile.dev.debug="full"' --config 'profile.dev.package."*".debug="full"' <command>
```

Cargo applies `--config` settings over the profiles in `Cargo.toml`, so nothing committed changes. It works the same with and without mise. The rebuilt files replace the lighter ones in `target/debug/`, and the next ordinary build replaces them again. `CONTRIBUTING.md` says so, and says to run `target-clean` afterwards if the folder grew. The plan step checked this in a scratch copy that had the lighter profile: `sidereal-plugin-abi` built with `-C debuginfo=line-tables-only` without the override, and with `-C debuginfo=2` with it.

Alternatives:

- **The `CARGO_PROFILE_DEV_DEBUG=full` environment variable alone.** It covers our own crates, but Cargo has no environment variable for a package override, so dependencies stay without debug information.
- **A committed `debugging` profile.** `cargo build --profile debugging` is easy to find, but writes a second full set of output to `target/debugging/`, which this change exists to avoid.

### 3. sccache stays a personal option for build time, not for disk (D2)

The measurements show sccache does not shrink `server/target/`: Cargo still writes every compiled crate into each worktree, and sccache only copies cached output there. On top, sccache keeps its own cache, 236 MiB for this workspace with the lighter profile. With N worktrees, disk use is N × the build folder, plus one cache. sccache therefore cannot help the problem this change solves. What it does buy is time. At the same path, a warm cache cut the build from 53 s to 18 s. In a second checkout at another path, which is the case for a new worktree, it served 85% of the cacheable calls from the cache and took 40 s. Those path-dependent misses also grew the cache, from 236 MiB to 294 MiB.

So the project does not configure sccache. `CONTRIBUTING.md` names it as a personal choice for faster first builds in new worktrees, set in the contributor's own environment or `mise.local.toml` (`RUSTC_WRAPPER=sccache`), with its cache size limit (`SCCACHE_CACHE_SIZE`, 10 GiB by default).

### 4. One build folder per worktree; a shared folder stays personal (D4)

The project sets no `build.target-dir` and no `CARGO_TARGET_DIR`. `CONTRIBUTING.md` describes a shared folder as a personal option: it stores dependencies once, but Cargo locks the folder for each build, so parallel builds in different worktrees wait for each other. Branches with different dependencies or Rust releases also replace each other's output, so each switch rebuilds.

### 5. `target-clean` deletes the whole folder once it is over the limit (D3)

The task is `[tasks.target-clean]` in `mise.toml`. It runs one POSIX `sh` script, `sh server/scripts/target-clean.sh`, and `CONTRIBUTING.md` lists that command in the task table for contributors without mise. The script:

1. Finds the repo root from its own path, so it works whatever folder it starts in, and only ever looks at `<root>/server/target`.
2. Reads `SIDEREAL_TARGET_LIMIT_MIB`, default 4096. Anything but a whole number stops it with an error that names the variable, before it measures anything.
3. Stops with an error, deleting nothing, if `server/target` is a symbolic link. Deleting the link would free nothing, and following it could delete a folder that other checkouts share.
4. Prints that there is nothing to delete if the folder is missing.
5. Measures the folder with `du -sk`, which is POSIX, and rounds up to whole MiB.
6. Deletes the folder with `rm -rf` when it is larger than the limit, and keeps it otherwise. It prints the size, the limit and which of the two it did.

The script never calls `cargo clean`: Cargo would follow `CARGO_TARGET_DIR` or a personal `build.target-dir`, and could delete a shared folder.

The default limit, 4096 MiB, is a little over twice the lighter profile's 1.8 GiB. A worktree can hold one full build plus one older generation, which is what a dependency bump or a new Rust release leaves behind. A third generation pushes it over the limit. `SIDEREAL_TARGET_LIMIT_MIB` lets a machine with less disk choose a lower limit, the same way `SIDEREAL_DB_PORT` lets a contributor choose the fixture's port.

Alternatives:

- **Delete only outdated files** (as `cargo-sweep` does). It keeps a warm build but needs a tool the project does not ship, or a reimplementation of Cargo's file layout, which changes between Rust releases.
- **`cargo clean --workspace`,** which keeps dependencies. It frees only our own crates, a small share of the folder.

### 6. No other file changes

The CI workflow, the `Dockerfile` and the scripts behind `dev`, `e2e`, `instance-verify` and `external-db-verify` keep their commands: they run Cargo, and Cargo reads the profile from the manifest. No decision here needs an ADR: each one is a few lines of configuration or documentation, and undoing any of them costs one pull request.

## Risks / Trade-offs

- [A contributor needs a debugger with local variables, and finds none] → `CONTRIBUTING.md` gives the override in decision 2, next to why the profile is lighter.
- [`target-clean` runs while a build in the same worktree is running, and breaks that build] → `CONTRIBUTING.md` says to run it only when no build runs in that worktree. Rerunning the broken build fixes it. Each worktree has its own folder, so other worktrees are never affected.
- [Deleting the folder makes that worktree's next build start from scratch] → this happens only above the limit, at most about one cold build's time. It is the trade-off D3 accepted.
- [The first build after the change rebuilds everything, on every machine and in CI] → once. The CI cache key changes with `Cargo.toml`, so CI saves a new, smaller cache after its first run.
- [The measured sizes depend on today's dependencies and Rust release] → the spec checks the profile settings, not a size (D5). The pull request records the numbers and the commands, so anyone can repeat them.

## Migration & Rollback

No data or runtime changes. Contributors pull the change; their next build rebuilds once with the new profile. Output built under the old profile stays in `server/target/` until `target-clean` or `cargo clean` deletes it, so the first `target-clean` after the change is likely to delete the folder. Rollback is reverting the pull request.
