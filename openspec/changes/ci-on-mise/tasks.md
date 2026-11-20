<!-- The yq checks use yq 4, which mise.toml does not name. `mise exec yq@4 -- yq` installs it for one command, as the ci spec's Purpose asks a reviewer to do. -->

## 1. The ci workflow installs from mise.lock

Every task in this section edits `.github/workflows/ci.yml`, so they run one at a time. Each job gets the install step from design.md D-1 straight after `actions/checkout`: `jdx/mise-action@v5` with `install_args: --locked` and no `cache` key, so mise-action's default cache stays on (D-2).

- [ ] 1.1 [easy] The `compose` job's steps are `actions/checkout`, then `jdx/mise-action`, and its one run step is `mise exec -- just check-compose` (verify: `test "$(mise exec yq@4 -- yq '.jobs.compose.steps[] | (.uses // .run) | sub("@.*", "")' .github/workflows/ci.yml | tr '\n' ' ')" = "actions/checkout jdx/mise-action mise exec -- just check-compose "`)
- [ ] 1.2 [easy] The `web` and `openspec` jobs each have the steps `actions/checkout`, `jdx/mise-action`, then one run step, `mise exec -- just check-web` and `mise exec -- just check-openspec` (verify: `test "$(mise exec yq@4 -- yq '(.jobs.web, .jobs.openspec).steps[] | (.uses // .run) | sub("@.*", "")' .github/workflows/ci.yml | tr '\n' ' ')" = "actions/checkout jdx/mise-action mise exec -- just check-web actions/checkout jdx/mise-action mise exec -- just check-openspec "`)
- [ ] 1.3 [medium] The `server` job installs through `jdx/mise-action`, caches cargo output with `Swatinem/rust-cache` and `cmd-format: mise exec -- {0}` (D-3), and runs its fixture and gate steps through mise, with `mise exec -- just check-server` as the only `check-` recipe (verify: `test "$(mise exec yq@4 -- yq '.jobs.server.steps[] | select(has("uses")) | .uses | sub("@.*", "")' .github/workflows/ci.yml | tr '\n' ' ')$(mise exec yq@4 -- yq '.jobs.server.steps[] | select(has("run")) | .run' .github/workflows/ci.yml | grep 'just check-')" = "actions/checkout jdx/mise-action Swatinem/rust-cache mise exec -- just check-server"`)
- [ ] 1.4 [medium] The `e2e` job's steps keep their order and comments, with `jdx/mise-action` in place of the Nix step, `cmd-format: mise exec -- {0}` on its cargo cache, and `mise exec --` in place of every `nix develop --command` (D-5) (verify: `test "$(mise exec yq@4 -- yq '.jobs.e2e.steps[] | select(has("uses")) | .uses | sub("@.*", "")' .github/workflows/ci.yml | tr '\n' ' ')" = "actions/checkout jdx/mise-action Swatinem/rust-cache actions/upload-artifact "`)
- [ ] 1.5 [easy] The `lets-spec` job's steps are `actions/checkout`, `jdx/mise-action`, then `mise exec -- lets sync --check`, and its comment says it checks against the release `mise.toml` names and that `mise exec` runs no `enter` hook (verify: `test "$(mise exec yq@4 -- yq '.jobs.lets-spec.steps[] | (.uses // .run) | sub("@.*", "")' .github/workflows/ci.yml | tr '\n' ' ')" = "actions/checkout jdx/mise-action mise exec -- lets sync --check "`)
- [ ] 1.6 [easy] No line of `ci.yml` mentions Nix, and every run line that calls `cargo`, `pnpm`, `just`, `lets` or `openspec` calls it through `mise exec --` (verify: `! grep -n -i 'nix' .github/workflows/ci.yml && ! mise exec yq@4 -- yq '.jobs[].steps[] | select(has("run")) | .run' .github/workflows/ci.yml | grep -nE '(^|[ (])(cargo|pnpm|just|lets|openspec) ' | grep -v 'mise exec -- '`)

## 2. The docs say where CI gets Rust

These tasks touch only `AGENTS.md` and `CONTRIBUTING.md`, and change only the sentences design.md D-7 names. The wider docs rewrite is not part of this change.

- [ ] 2.1 [parallel] [easy] `AGENTS.md`'s "Rust follows the latest stable release" bullet says `flake.lock` decides the release in the Nix shell and the `nix` workflow, and rustup decides it in the `ci` workflow and on the mise route (verify: `! grep -zqE 'in the Nix shell and[[:space:]]+CI\.' AGENTS.md && grep -q 'nix. workflow' AGENTS.md`)
- [ ] 2.2 [parallel] [easy] `CONTRIBUTING.md`'s "Without Nix" Rust note and the Rust row of its versions table say the `ci` workflow installs Rust's stable channel through mise and rustup decides the release (verify: `! grep -nE 'takes Rust from .flake\.lock.|the .ci. and .nix. workflows' CONTRIBUTING.md`)

## 3. The change passes its gates [depends-on: 1, 2]

- [ ] 3.1 [easy] The local gate passes on the branch, including strict validation of this change (verify: `mise exec -- just check`)
- [ ] 3.2 [medium] In a scratch copy of the repo under `.workspace/`, adding a tool to `mise.toml` without updating `mise.lock` makes the install that every job runs fail. Delete the copy afterwards; nothing from it is committed (verify: `rm -rf .workspace/stale-lock && git worktree add -q --detach .workspace/stale-lock && sed -i 's/^\[tools\]$/[tools]\njq = "1"/' .workspace/stale-lock/mise.toml && (cd .workspace/stale-lock && mise trust -q && ! mise install --locked); s=$?; git worktree remove --force .workspace/stale-lock; test "$s" -eq 0`)
- [ ] 3.3 [medium] After the branch is pushed, every job of the `ci` workflow passes on pull request #511: `compose`, `server`, `web`, `openspec`, `e2e` and `lets-spec` (verify: `gh pr checks 511`)

## Deviations Ledger

<!-- Dated notes on small departures from the plan, written during apply. Stop and ask the user about a large one. -->
