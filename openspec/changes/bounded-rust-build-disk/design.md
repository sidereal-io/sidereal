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

- **Primary test seam:** a new POSIX `sh` check script, `server/scripts/target-clean-check.sh`. It builds each case of the `target-clean` spec in a scratch copy of the repo layout, under `mktemp -d`, runs the clean-up script there, and asserts the exit status, the output and what is left on disk. It needs no Rust toolchain and no Docker. It follows the pattern of `scripts/dev-supervisor-check.sh`, which also makes its fixture with `mktemp -d` (line 15), and like that script runs by hand: `sh server/scripts/target-clean-check.sh`.
  - Fixture files are written with `dd if=/dev/urandom`, so a sparse file or a compressing file system cannot make them measure smaller than 2 MiB.
  - Cases that need a failing or unusual system command put a stub first on `PATH` for the clean-up script only: a `du` that exits non-zero, an `rm` that exits non-zero, a `mount` that lists `server/target` as a mount point, a `mount` that lists `server/target/debug/cache`, a `mount` that lists `server`, and a `mount` that exits non-zero. A real mount needs root, so the stub `mount` stands in for it. The stub prints the fixture's physical path, from `pwd -P`, in the form that `mount` uses.
  - One case builds the fixture under a folder whose name holds a space, and expects a refusal.
  - One real run through mise covers the task's entry point: `SIDEREAL_TARGET_LIMIT_MIB=not-a-number mise run target-clean` in the real checkout. It deletes nothing, because the script checks the limit before it measures anything (decision 5, step 2). It fails if the task cannot find the script, for example because it sets the wrong `dir`, or if the task overrides the caller's `SIDEREAL_TARGET_LIMIT_MIB`.
- **Existing seams reused:** the profile is checked the way its spec scenarios say: by reading `server/Cargo.toml`, and by verbose builds in a new empty `CARGO_TARGET_DIR`, whose compiler commands show the `-C debuginfo=` value. Cargo adds `-C strip=debuginfo` to a dependency's command when its debug information is off. That option is not a `-C debuginfo=` value, so the checks match only `-C debuginfo=`. `cargo test --no-run -v -p sidereal-plugin-abi` covers a workspace crate. `cargo build -v -p libc` covers a dependency, because `sidereal-plugin-abi` has no dependencies of its own. Each build writes its output to a log file and is checked for success before the log is searched, so a failed build fails the check. `mise run check` proves the gate still passes with the new profile, and `openspec validate` covers the spec deltas.

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

Cargo applies `--config` settings over the profiles in `Cargo.toml`, so nothing committed changes. It works the same with and without mise. Cargo keeps the full-debug output next to the lighter output, in folders named by a hash of the settings. So `server/target/` grows by about one full-debug build, and the next ordinary build does not shrink it. `CONTRIBUTING.md` says so, and says to run `target-clean` after the debugging session. The plan step checked this in a scratch copy that had the lighter profile:

- `sidereal-plugin-abi` built with `-C debuginfo=line-tables-only` without the override, and with `-C debuginfo=2` with it.
- A dependency's compiler command had no `-C debuginfo` flag without the override, and `-C debuginfo=2` with it.
- After a full-debug build and an ordinary build, `target/debug/build/` held two hash folders for each package, and `target/debug/incremental/` held two folders for each workspace crate.

Alternatives:

- **The `CARGO_PROFILE_DEV_DEBUG=full` environment variable alone.** It covers our own crates, but Cargo has no environment variable for a package override, so dependencies stay without debug information.
- **A committed `debugging` profile.** `cargo build --profile debugging` is easy to find. It writes a second full set of output to `target/debugging/`, as the override does, but a committed profile makes that cost a project setting that invites routine use. The override stays a one-off for a debugging session.

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
5. Stops with an error, deleting nothing, if `server`, `server/target` or any folder inside `server/target` is a mount point. `rm -rf` crosses mount points, and POSIX gives it no option to stay on one file system, so a bind-mounted folder that other worktrees share would lose its contents. A bind mount from the same file system has the same device number as its parent, so device checks such as `find -xdev` cannot see it. The script reads the system mount table instead:
   - It resolves the folder's physical path with `cd -P server/target && pwd -P`, and the physical path of `server` with `cd -P server && pwd -P`.
   - It runs `mount` with no arguments. Linux prints `<source> on <mount point> type <fs> (<options>)`, and macOS and the BSDs print `<source> on <mount point> (<fs>, <options>)`. Both list every mount, bind mounts included.
   - It refuses when any line holds ` on <path> ` or ` on <path>/`. That covers the folder itself and every folder inside it, on any file system.
   - It refuses when any line holds ` on <server path> `. A bind mount at `server` would make `server/target` another checkout's build folder, and `pwd -P` does not see through a bind mount. A mount at the repo root, such as a container that mounts the whole checkout, is not refused: the whole checkout is then the same tree, so the folder is still this checkout's own.
   - It refuses when `mount` fails or prints nothing, so it never deletes when it cannot tell.
   - It refuses when the physical path holds a space, a tab, a newline or a backslash. `mount` prints those characters escaped, for example `\040` for a space, so the script cannot compare the path reliably. It refuses in doubt rather than guess.
6. Measures the folder with `du -sk`, which is POSIX and reports the disk space allocated, and rounds up to whole MiB. If `du` fails, the script stops with an error and deletes nothing.
7. Deletes the folder with `rm -rf` when it is larger than the limit, and keeps it otherwise. It prints the size, the limit and which of the two it did. If `rm -rf` fails, the script exits with an error that says the folder may be partly deleted.

The script never calls `cargo clean`: Cargo would follow `CARGO_TARGET_DIR` or a personal `build.target-dir`, and could delete a shared folder.

The default limit, 4096 MiB, is a little over twice the lighter profile's 1.8 GiB. A worktree can hold one full build plus one older generation, which is what a dependency bump or a new Rust release leaves behind. A third generation pushes it over the limit. `SIDEREAL_TARGET_LIMIT_MIB` lets a machine with less disk choose a lower limit, the same way `SIDEREAL_DB_PORT` lets a contributor choose the fixture's port.

The limit is a threshold that applies only when `target-clean` runs. Between runs, a worktree can grow past it, so N worktrees can hold more than N × the limit. To choose a limit, keep room for the builds that run at the same time, about 2 GiB each with the lighter profile, then divide what is left by the number of worktrees. `CONTRIBUTING.md` gives the same guidance. A scheduler that checks that a worktree is idle before it runs `target-clean` can still race a build that starts during the clean-up. That build fails, and rerunning it fixes it.

Alternatives:

- **Delete only outdated files** (as `cargo-sweep` does). It keeps a warm build but needs a tool the project does not ship, or a reimplementation of Cargo's file layout, which changes between Rust releases.
- **`cargo clean --workspace`,** which keeps dependencies. It frees only our own crates, a small share of the folder.

### 6. No other file changes

The CI workflow, the `Dockerfile` and the scripts behind `dev`, `e2e`, `instance-verify` and `external-db-verify` keep their commands: they run Cargo, and Cargo reads the profile from the manifest. Each route that D6 lists has a check that runs it with the new profile:

| Route | What runs it |
|---|---|
| Debug build, `cargo test`, `cargo clippy --all-targets`, `mise run check` | Task 4.1 runs the gate, and CI's `server` job runs `mise run check-server` on every pull request. |
| `dev` and `e2e` | Both start the server with `cargo run -p sidereal-server` (`scripts/dev.sh`, and the `webServer` entry in `web/playwright.config.ts` that `scripts/e2e.sh` uses). Task 4.1 builds the same package. CI's `e2e` job builds it and runs `mise run e2e` on every pull request. |
| Release build, `instance-verify`, `external-db-verify` | Both checks build the app image, which runs `cargo build --release` in the `Dockerfile`. The plan sets no release setting, and the scenario "Only the dev profile is set" checks that. |
| Hand-install route without mise | It runs the same scripts and Cargo commands, which read the same manifest. The check script runs the plain `target-clean` command with no mise. | No decision here needs an ADR: each one is a few lines of configuration or documentation, and undoing any of them costs one pull request.

## Risks / Trade-offs

- [A contributor needs a debugger with local variables, and finds none] → `CONTRIBUTING.md` gives the override in decision 2, next to why the profile is lighter.
- [`target-clean` runs while a build in the same worktree is running, and breaks that build] → `CONTRIBUTING.md` says to run it only when no build runs in that worktree. A scheduler's idle check cannot rule out a build that starts during the clean-up. Rerunning the broken build fixes it. Each worktree has its own folder, so other worktrees are never affected.
- [`server`, `server/target`, or a folder inside `server/target`, is a mount point, and `rm -rf` would empty another file system or another checkout's build folder] → the script reads the mount table, refuses it and deletes nothing (decision 5, step 5).
- [A checkout whose path holds a space, tab, newline or backslash can never be cleaned by `target-clean`] → the script says why it refuses. `CONTRIBUTING.md` says so, and gives the safe next step: move the checkout to a path without those characters. Deleting `server/target` by hand would skip the mount check, so `CONTRIBUTING.md` does not suggest it.
- [Many worktrees each stay just under the limit, or grow past it between runs, and still fill the disk] → the limit is a per-worktree threshold, not a cap. `CONTRIBUTING.md` says how to choose a limit from the free disk, the builds that run at once and the number of worktrees (decision 5).
- [Deleting the folder makes that worktree's next build start from scratch] → this happens only above the limit, at most about one cold build's time. It is the trade-off D3 accepted.
- [The first build after the change rebuilds everything, on every machine and in CI] → once. The CI cache key changes with `Cargo.toml`, so CI saves a new, smaller cache after its first run.
- [The measured sizes depend on today's dependencies and Rust release] → the spec checks the profile settings, not a size (D5). The pull request records the numbers and the commands, so anyone can repeat them.

## Migration & Rollback

No data or runtime changes. Contributors pull the change; their next build rebuilds once with the new profile. Output built under the old profile, about 3.1 GiB, is under the default limit, so `target-clean` keeps it. A contributor short of disk runs `SIDEREAL_TARGET_LIMIT_MIB=0 mise run target-clean` once, before the first build after this change. `CONTRIBUTING.md` says so. Rollback is reverting the pull request.
