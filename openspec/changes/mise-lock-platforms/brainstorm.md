<!-- Plain language: Write questions and answers so a reader without the conversation can understand each decision and its source. Keep the confirmation marker exactly as written. -->

## Story

#503 mise-lock-platforms — mise.lock pins the same release on Linux and macOS, https://github.com/sidereal-io/sidereal/issues/503

## Decisions

| # | Question | Answer | Source |
|---|---|---|---|
| D1 | Should this story add anything to the weekly update (#501), or rely on the check? | Rely on the check and add no `lockfile_platforms` setting: `mise lock` keeps the lock's existing platforms, and #501's pull request changes `mise.lock`, so the check runs on it. | Assumption |
| D2 | Which `mise.lock` entries must have `linux-x64`, `linux-arm64` and `macos-arm64` tables? | Every `[[tools.*]]` entry except Rust, which records the `stable` channel and has no platform tables; a tool added later without them fails until its change names the exception. | Assumption |
| D3 | Where should the platform check run? | Only in the `mise` workflow, from a script under `scripts/` that a contributor can also run by hand; not in `just check`. | Assumption |
| D4 | What counts as passing? | The check fails with an error that names each tool and each missing platform, and passes on the current `mise.lock`. | Assumption |

## Changes from the story

None. The packet's second scope item, that the weekly update (#501) keeps every platform, is met by the check running on #501's pull request (D1), not by a change to #501.

Confirmed by user: no
Confirmed by lets-go: yes (the user did not decide)
