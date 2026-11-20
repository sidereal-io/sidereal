<!-- Plain language: Lead with the problem and resulting behavior. Give a reader enough context to decide whether the scope solves that problem. Keep the required headings. -->

## Why

`mise.lock` today records a download address and checksum for Linux x64, Linux arm64 and macOS arm64 for every tool except Rust. Nothing checks that it keeps them. If a later lock update drops one of those platforms, a contributor on that system gets a release that mise resolves on first install, not the one CI tested. This story closes that gap now, while the mise route is new and its lock has only one update path: the weekly update (#501).

## What Changes

- The mise check (the `mise` workflow) gains a step that fails when an entry in `mise.lock` lacks a `linux-x64`, `linux-arm64` or `macos-arm64` table. Its error names each tool and each missing platform (D4).
- The step runs a script under `scripts/` that a contributor can also run by hand. It does not run in `just check` (D3).
- Every `[[tools.*]]` entry must have the three tables, except Rust, which records the `stable` channel and has no platform tables. A tool added later without them fails the check until its change names the exception (D2).
- The weekly update (#501) is not changed. Its pull request changes `mise.lock`, so the mise check, and this step with it, runs on that pull request (D1).
- The script passes on the current `mise.lock`. This change writes no lock entries; #497 already wrote them.

Out of scope: native Windows. The lock's Windows tables stay, but the check does not require them, and they do not make Windows supported.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `dev-environment`: adds a requirement that the mise check fails when `mise.lock` lacks a supported platform for a tool, with Rust as the one exception.

## Impact

- `.github/workflows/mise.yml`: one new step in the `mise-check` job. The workflow's path filter already includes `mise.lock`, so no trigger changes.
- `scripts/`: one new script that reads `mise.lock`.
- `openspec/specs/dev-environment/spec.md`: one added requirement.
- No change to `mise.toml`, `mise.lock`, `flake.nix`, the Nix route, or `just check`.
