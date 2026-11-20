<!-- Plain language: Help the next implementer find the approach, understand the choices and act on the plan. Explain necessary technical terms. Put each risk beside its mitigation and each open question beside the decision it affects. -->

## Context

- `mise lock` writes `mise.lock` in a fixed layout. Each tool has one `[[tools.<name>]]` header, then one `[tools.<name>."platforms.<platform>"]` table per platform. `<name>` is bare (`gh`) or quoted (`"github:Fission-AI/OpenSpec"`).
- Today every entry except `[[tools.rust]]` has seven platform tables: `linux-x64`, `linux-arm64`, both `-musl` variants, `macos-x64`, `macos-arm64` and `windows-x64`. Rust has only `[tools.rust.options]`.
- The mise check (`.github/workflows/mise.yml`, job `mise-check`) runs on `ubuntu-latest` and already has shell steps after the install. Its path filter already includes `mise.lock`.
- Scripts in `scripts/` are POSIX `sh` with `set -eu`, a `fail()` helper and a `<script-name>: FAILED: ...` message on standard error (see `scripts/compose-config-check.sh`).
- A contributor on macOS may have only the system `python3` (3.9, no `tomllib`) and no `yq`. A contributor may run the script outside `mise exec`.

## Goals / Non-Goals

**Goals:**

- One script, `scripts/check-mise-lock-platforms.sh`, that the mise check and a contributor run the same way.
- Needs only `sh` and `awk`, which every supported system has, so it runs before any tool is installed.
- Reports every missing platform for every tool in one run, not only the first.

**Non-Goals:**

- Checking that each table has a `url` and a `checksum`. `mise install --locked` already fails on a broken table on Linux x64, and the existing spec scenario covers a reader's check.
- Parsing TOML in general. The script reads only the two header forms `mise lock` writes.
- Running in `just check` (D3).

## Test Seams

- **Primary test seam:** the script's exit status and output, run against copies of `mise.lock`. The script takes an optional path argument, default `mise.lock` at the repo root, so a test can point it at a changed copy in `.workspace/` without touching the committed lock.
- **Existing seams reused:** the mise check itself, which runs the script on this change's pull request against the committed lock.

No automated test suite runs the script: D3 keeps it out of `just check`. The build step proves each spec scenario by running the script on a fixture copy and recording the command and result in `tasks.md`.

## Scrutiny Tier & Blast Radius

- **Tier:** low
- **Blast radius:** one new script and one new step in the `mise-check` job. It reads `mise.lock` and changes nothing. A false failure blocks pull requests that touch the mise route until fixed; it cannot affect the app, the Nix route or `just check`.

## Decisions

1. **Use `sh` and POSIX `awk`, not a TOML parser.** The script uses POSIX string functions (`sub` and `split`), rather than GNU regex capture group arrays, to extract tool names from `^\[\[tools\.(.+)\]\]$` and `^\[tools\.(.+)\."platforms\.([^"]+)"\]$`. Quotes are stripped from quoted tool identifiers (such as `"github:Fission-AI/OpenSpec"`) upon parsing so that table lookups and error reports use canonical unquoted names. Alternatives: `python3 -c 'import tomllib'` fails on macOS's system Python 3.9; `yq` or `taplo` would need another pinned tool. The cost is that a change in `mise lock`'s layout could make the script misread the lock (see Risks).
2. **The exception list lives in the script, as one variable.** It holds `rust` only (D2). A later tool that has no platform tables changes this variable and the spec requirement in the same pull request. Alternative: read the exceptions from `mise.toml` comments or a setting. That adds a second place to look, for one entry.
3. **The required platforms live in the script, as one variable:** `linux-x64 linux-arm64 macos-arm64`. They match the systems `flake.nix` supports. Alternative: read `systems` from `flake.nix`. That couples the mise route to the Nix route, which is meant to go away.
4. **Report all gaps, then fail once.** For each tool that lacks tables, print one line naming the tool and its missing platforms, as `check-mise-lock-platforms: FAILED: <tool> lacks <platform> <platform>`. In the workflow, also print each line as a GitHub `::error::` annotation, as the Rust channel step does, so it shows on the pull request. The script detects GitHub Actions through `GITHUB_ACTIONS=true`.
5. **Fail when the lock has no entries at all.** A lock the script cannot read, or a layout change that hides every header, must not pass silently. This also covers a missing file.
6. **Run the step before the install.** The step needs nothing mise installs, and failing first gives a clearer error than a later install failure on another platform.

No decision here is costly to reverse, so none needs an ADR.

## Risks / Trade-offs

- [A future `mise lock` release changes the header layout, so the script finds no entries or no tables] → Decision 5 makes the check fail loudly rather than pass. The fix is a pattern change in one script.
- [The weekly update (#501) drops a platform] → Its pull request changes `mise.lock`, so the mise check runs this step and fails (D1). The pull request cannot merge green.
- [Adding a new tool via `mise lock` on a single platform omits the other supported platforms] → The check fails in CI on pull requests touching `mise.lock`. The contributor can run `mise lock --platform <platform>` for each required platform or configure `lockfile_platforms` under `[settings]` in `mise.toml`.
- [A contributor adds a tool that mise cannot lock for one of the platforms, such as a tool with no macOS arm64 build] → The check fails, as intended. The contributor adds the tool to the exception list and the spec in the same pull request, so the gap is a recorded decision.
- [The script and the existing spec scenario "The lock covers the supported systems" list tools differently: the scenario names six tools, the script checks every entry but Rust] → They agree today. The new requirement states the general rule, and the old scenario remains a reader's check for the current six.

## Migration & Rollback

N/A. The script and the workflow step are new. To roll back, revert the commit that adds them.
