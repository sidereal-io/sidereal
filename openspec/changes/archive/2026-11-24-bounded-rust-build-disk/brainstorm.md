## Story

#543 bounded-rust-build-disk — repeated Rust builds across worktrees keep a bounded disk footprint.
https://github.com/sidereal-io/sidereal/issues/543

## Decisions

| # | Question | Answer | Source |
|---|---|---|---|
| D1 | Where does the main fix live: a committed build profile, a contributor's own opt-in setting, or both? | Both. A lighter build profile committed in `server/Cargo.toml` (line tables only for our crates, no debug information for dependencies) applies to every contributor and to CI. `CONTRIBUTING.md` says how to turn full debug information back on for a debugging session. | User |
| D2 | How does the story judge sccache, which is not installed on the development machine? | Measure the options, sccache included, by total disk use (build output plus any cache of its own), and decide from the measurements. The decision and its numbers go in the design and the pull request. sccache never becomes a requirement for contributors. | User |
| D3 | How does a worktree's build output stay small over time, given that Cargo never deletes old output? | A mise task deletes `server/target/` only when it is over a documented size limit. `CONTRIBUTING.md` documents it and gives the plain command for contributors without mise. A contributor or an outside scheduler runs it; `mise run check` does not. | User |
| D4 | Does the project share one build folder across all worktrees? | No, not as a project setting: Cargo locks the folder during a build, so parallel builds would wait for each other, and branches with different dependencies would keep rebuilding each other's output. `CONTRIBUTING.md` mentions it only as a personal option, with that trade-off. | User |
| D5 | How does the pull request prove the result? | Before-and-after sizes of `server/target/` for the same commands, recorded in the pull request, plus a spec scenario a reviewer can check by reading `server/Cargo.toml`. No size check in `mise run check`. | User |
| D6 | Which builds and routes must keep working? | The debug build, `cargo test`, `cargo clippy --all-targets` and the release build; `mise run check`, `dev`, `e2e`, `instance-verify` and `external-db-verify`; and the hand-install route without mise. | Evidence: #543 issue body (Scope, in 4) |
| D7 | Does the story change how CI caches Cargo output? | No, beyond keeping CI's cache working with the new profile. | Evidence: #543 issue body (Scope, out 1) |

## Changes from the story

- **Shared build folder:** Scope item 3 listed it as a candidate. It is ruled out as a project setting (D4), and stays only as a documented personal option.
- **Clean-up rule:** Scope item 3 left open whether a retention rule was needed. The story now adds a size-limited clean-up task that runs outside the gate (D3).
- **No automated size check:** proof is measured sizes in the pull request and a spec scenario on the profile settings, not a check in `mise run check` (D5).

Confirmed by user: yes
