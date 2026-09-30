## Why

The `server` job in `v2.yml` installs Rust through rustup and repeats the `just check` steps by hand. CI and the local gate can therefore drift apart. They already have: CI runs `cargo build`, and `just check` does not. Landing this before #301 lets the web gates reach CI through `just`, with no second tool setup to build.

## What Changes

- A new `check-server` recipe in the `justfile` holds today's server gate. The gate is the format check, clippy with warnings denied, the tests, and the dependency-direction lint.
- `just check` runs `check-server`. Contributors see no change.
- The `server` job installs Nix the same way `nix.yml` does. It then runs `nix develop --command just check-server`, and that is its only check step.
- The `server` job loses its `rustup show` step and its five hand-written check steps.
- CI stops running `cargo build` as a separate check. Clippy and the tests already compile every target, including the `sidereal-server` binary.
- `v2.yml` also runs on changes to `flake.nix`, `flake.lock`, and `nix/**`. A change to the shell can change the result of the v2 checks.
- `Swatinem/rust-cache` stays. It gains one setting, so it reads the Rust version from the shell and not from rustup.
- Unchanged: the v0.10.x workflows, `nix.yml`, and the `pull_request`-only trigger. #320 adds runs on pushes to `main`, #301 adds the web gates, and #288 adds a Nix store cache.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `ci`: the requirement for the `server` job gains the Nix files as triggers and no longer lists a build as a check. A new requirement says that every v2 job runs one `just` recipe inside the Nix development shell.

Issue #289 names `dev-environment` as the spec to change. This change uses `ci` instead. The `ci` spec did not exist when #289 was first written, and it already holds the `server` job's requirement. The `dev-environment` spec needs no edit: its gate, `just check`, runs the same four checks as before.

## Impact

- **Code and config:** `justfile` and `.github/workflows/v2.yml`.
- **Docs:** `server/README.md` lists the `just` recipes, so it gains `check-server`.
- **CI time:** the `server` job takes about 20 seconds today. After this change it takes about 110 seconds, measured on this change's first run. Installing Nix and building the shell account for about 57 of those seconds. The rest is a cold compile and saving the cargo cache.
- **Dependabot:** the weekly `flake.lock` pull request now runs the `server` job as well as the flake check.
- **Open work:** #320 and #301 both edit `v2.yml` after this change. #301 adds a `check-web` recipe beside `check-server`.
