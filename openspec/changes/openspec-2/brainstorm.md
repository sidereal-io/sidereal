**Why:** Both routes still run OpenSpec 1.13.2, a major version behind, and the mise route needs a second dependency lock in `.mise/locks/` for it; the 2.1.1 release binary is pinned by URL and checksum like every other tool, and #505 has made every spec pass the strict check on 2.1.1.

## Story

#506 openspec-2 — both routes run the OpenSpec 2.1.1 release binary, https://github.com/sidereal-io/sidereal/issues/506

## Decisions

| # | Question | Answer | Source |
|---|---|---|---|
| D1 | Is the Nix part in scope? | Yes. #502, which removes the Nix route, is still open and `flake.nix` still exists, so `flake.nix` packages the 2.1.1 binary too. | Evidence: `gh issue view 502` (OPEN) and `flake.nix`, 2026-11-19 |
| D2 | How does mise name and lock OpenSpec? | `mise.toml` names `"github:Fission-AI/OpenSpec" = "2.1.1"` in place of `"npm:@fission-ai/openspec"`. `mise lock` then records a URL and checksum for `linux-x64`, `linux-arm64`, `macos-arm64` and the other release platforms, and the checksums match the release's `SHA256SUMS`. | Evidence: trial `mise lock` in a throwaway copy under `.workspace/`, 2026-11-19 |
| D3 | What does the Nix package of the binary need? | One `fetchurl` per system from the release (`openspec-linux-x64.tar.gz`, `openspec-linux-arm64.tar.gz`, `openspec-darwin-arm64.tar.gz`) with the hash from `SHA256SUMS`. The Linux binaries link to glibc dynamically, so they need `autoPatchelfHook`. They are Bun-compiled, so the build must not strip them. The shell keeps `OPENSPEC_NO_UPDATE_CHECK=1`, which 2.1.1 still reads. | Evidence: `file`, `ldd` and `strings` on the 2.1.1 `linux-x64` binary, 2026-11-19 |
| D4 | Does OpenSpec 2's `openspec update` cleanup or skill layout clash with what `lets sync` writes? | No clash in the layout: `openspec update --dry-run` on 2.1.1 would rewrite the 8 `openspec-*` skills in `.agents/skills/` and add `.agents/skills/.openspec-target`, and deletes nothing. `lets sync --check` exits 0 with 2.1.1 on `PATH`. The existing rule "never run `openspec update` by hand" stays. | Evidence: 2.1.1 `update --dry-run --json` and `lets sync --check` in a throwaway copy, 2026-11-19 |
| D5 | Which `dev-environment` requirements change? | "The Nix shell provides the OpenSpec CLI" no longer names the lock file as the only pin. "The mise lock pins every other tool's exact release" adds the `github:Fission-AI/OpenSpec` entry to the platforms scenario, since mise now downloads it as a file. "CI checks the mise route" drops the `.mise/` bullet. "mise sets the Let's Spec flow and keeps the OpenSpec CLI offline" says "a newer version" rather than "the npm registry". | Evidence: `openspec/specs/dev-environment/spec.md` |
| D6 | What proves the change is done? | `nix develop --command openspec --version` and `mise exec -- openspec --version` both print `2.1.1`; `mise install --locked` succeeds; `.mise/` is gone; `lets sync --check` exits 0; `just check` passes, with `check-openspec` now running 2.1.1. | Evidence: issue #506 scope, `justfile` `check-openspec` |
| D7 | What do the hand-install steps in `CONTRIBUTING.md` say? | Install OpenSpec 2.1.1 from its GitHub release, with `npm install -g @fission-ai/openspec@2.1.1` as the alternative. | Assumption |
| D8 | With 2.1.1 on `PATH`, `lets sync` warns on every shell entry, on both routes, that the skills were generated from 1.13.2. It exits 0 and writes nothing, but the spec and `CONTRIBUTING.md` say entering prints nothing. Keep, hold or filter the warning? | Keep it. Reword the mise "Nothing changed" scenario and `CONTRIBUTING.md` to allow exactly this warning, until a Let's Spec release names OpenSpec 2. | Assumption |
| D9 | The Let's Spec bundle carries `openspec` 1.13.2 with `lets` and `gh`, and the shell cannot change it. Put 2.1.1 ahead of it on `PATH`, or rebuild `lets` and `gh` without the bundle? | Put the 2.1.1 package ahead of the bundle on `PATH`, and add a check that `nix develop --command openspec --version` prints `2.1.1`. | Assumption |
| D10 | The `AGENTS.md` "Update Let's Spec on both routes" bullet, step 3 of the `CONTRIBUTING.md` upgrade steps and the `mise.toml` comment say to take the OpenSpec version from the Let's Spec release notes, which would undo this change. Reword them? | Yes. All three say OpenSpec is pinned on its own until a Let's Spec release names OpenSpec 2. | Assumption |
| D11 | About eight `dev-environment` requirements describe a `just skills` recipe and `scripts/skills.sh`, and neither exists any more. Reword them all here? | No. Reword only "The Nix shell provides the OpenSpec CLI" here, and file the stale `just skills` requirements as a new chore with `lets-shape`. | Assumption |

## Changes from the story

Split out: the stale `just skills` requirements in `dev-environment` go to a new chore (D11), because rewording them is not needed to move to OpenSpec 2.1.1. The chore's issue is filed once its packet is confirmed.

Confirmed by user: no
Confirmed by lets-go: yes (the user did not decide)
