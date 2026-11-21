## Story

#497 mise-dev-env — mise installs Sidereal's pinned tools and syncs the skills when you enter the repo, https://github.com/sidereal-io/sidereal/issues/497

## Decisions

| # | Question | Answer | Source |
|---|---|---|---|
| D1 | `flake.lock` pins an untagged Let's Spec commit (3a9e669), and mise installs only tagged releases. Which release do both routes use? | v0.8.0. The Nix route moves to it in this change, in its own commit with `flake.lock` and what `lets sync` wrote; `mise.lock` pins 0.8.0. | User |
| D2 | Should the flake input pin the release tag, so Dependabot cannot move it to an untagged commit? | Yes: `github:sidereal-io/lets-spec/v0.8.0`, until #502 removes the Nix route. | User |
| D3 | `mise.lock` records Rust as `stable`, with no exact release. What decides Rust on the mise route? | rustup's newest stable release at install time. The rule that no file names a Rust release stays, and the decision record says this. | User |
| D4 | Which files start the `mise` workflow on a pull request? | `mise.toml`, `mise.lock`, the npm sidecar under `.mise/`, the workflow's own file, `server/rust-toolchain.toml` and `.nvmrc`. The sidecar was added after the plan review: a deleted or hand-edited sidecar leaves `mise.lock` unchanged. | User |
| D5 | Should `mise.toml` set `OPENSPEC_NO_UPDATE_CHECK=1`, as the Nix shell does? | Yes, so both routes keep the OpenSpec CLI offline. | User |
| D6 | Which platforms does `mise.lock` cover? | Every platform `mise lock` writes by default, with no `--platform` flag and no `lockfile_platforms` setting. The Windows and musl entries are download records nobody uses. They do not make Windows supported: #496 still lists native Windows development as Won't. #503 narrows to what is left once Linux and macOS are in the lock. | User |
| D7 | `CONTRIBUTING.md`'s "Where versions are set" table says each tool version lives in one file. Does this change add `mise.lock` to it? | Yes. Add `mise.lock` to the rows for the tools it pins beside `flake.lock`, so the table is right from this merge until #502. | User |
| D8 | From the repo root, mise neither locks nor installs Rust from `server/rust-toolchain.toml`: it reads that file only inside `server/`. How does mise get Rust? | `mise.toml` names `rust = { version = "stable", components = "rustfmt,clippy" }`. `server/rust-toolchain.toml` stays for rustup users. The `mise` workflow fails when the two files name different channels. The spec's rule becomes "both files name the stable channel and no file names a release". | User |

## Changes from the story

- **Changed:** the packet said `mise.lock` pins "the same Let's Spec release `flake.lock` pins". `flake.lock` pins an untagged commit, so this change moves the Nix route to v0.8.0 and pins the flake input to that tag (D1, D2).
- **Added:** `mise.toml` sets `OPENSPEC_NO_UPDATE_CHECK=1` (D5).
- **Added:** the `mise` workflow also runs when `server/rust-toolchain.toml`, `.nvmrc` or a file under `.mise/` changes (D4).
- **Answered:** the packet's open question on Rust. `mise.lock` does not pin the exact Rust release (D3).
- **Moved in:** the packet left Linux and macOS entries in the lock to #503. This change's lock covers every platform `mise lock` writes by default, so #503 keeps only the workflow check for a missing platform and the weekly update that keeps every platform (D6).
- **Added:** `CONTRIBUTING.md`'s "Where versions are set" table names `mise.lock` beside `flake.lock` (D7).
- **Changed:** the packet said `mise.toml` takes Rust "from `server/rust-toolchain.toml`". mise reads that file only inside `server/`, so `mise.toml` names the `stable` channel itself, and the `mise` workflow checks the two files agree (D8).

Confirmed by user: yes
