<!-- Plain language: Write questions and answers so a reader without the conversation can understand each decision and its source. Keep the confirmation marker exactly as written. -->

## Story

#501 mise-tool-updates — tool pin updates, Let's Spec included, arrive as one passing pull request a week. https://github.com/sidereal-io/sidereal/issues/501

## Decisions

| # | Question | Answer | Source |
|---|---|---|---|
| D1 | Which tools does the weekly run update? | Every tool within the range `mise.toml` sets (plain `mise upgrade`: Node 26.x from `.nvmrc`, pnpm 12.x, gh 2.x), plus Let's Spec to its newest release (`mise upgrade --bump github:sidereal-io/lets-spec`). OpenSpec stays at 2.1.1, Rust stays on the `stable` channel, and major versions stay manual. | Assumption |
| D2 | A Let's Spec update in `mise.toml` leaves `flake.nix` on the old tag, which breaks "Both routes use one Let's Spec release". How is that handled? | By merge order. The workflow runs only from `main`, and #502 (which removes the Nix route) sits directly above this branch in the stack, so both land before any weekly run. The pull request says so. No spec exception. | Assumption |
| D3 | Which GitHub App gives the token that opens the weekly pull request? | The existing dependency-merge App, through `DEPENDABOT_MERGE_APP_CLIENT_ID` and `DEPENDABOT_MERGE_APP_PRIVATE_KEY`. The token asks only for `contents: write` and `pull-requests: write`. No new setup by an administrator. | Assumption |
| D4 | When does the workflow run? | Every Monday at 04:00 UTC, the slot Dependabot's entries use, and on demand from the Actions tab (`workflow_dispatch`). | Assumption |
| D5 | If the updated tools fail the gate, does the workflow still open or update the pull request? | Yes. It opens or updates the one pull request, and CI shows the failure there for Mo to act on. If nothing changed, it opens no pull request. If `mise lock` or `lets sync` fails, the workflow run fails and opens no pull request. The workflow does not run `mise run check` itself. | Assumption |
| D6 | How is acceptance shown, given the workflow cannot run from this branch before merge? | The update steps live in a POSIX `sh` script a tester runs by hand to see the changes it makes. The workflow's triggers and token permissions are checked by reading the file, as the `ci` spec's `yq` scenarios do. The exit demonstration is the first manual run after merge; the pull request records that as a limit. | Assumption |
| D7 | What stays out of scope? | Merging the weekly pull request by itself (the Could in #496) and removing Dependabot's `nix` entry (#502). | Evidence: #501 body (Scope, out) |
| D8 | Which documents describe the new route? | The `dev-environment` spec and `docs/dependency-updates.md`, beside Dependabot's npm, Cargo, Docker and Actions entries. The `ci` spec's "Each workflow grants only the token access it needs" covers the new workflow too. | Evidence: #501 body (Scope, in), `openspec/specs/ci/spec.md` |

## Changes from the story

- Changed: the weekly pull request is not promised to pass CI. It opens with no manual step and CI runs on it; when an update breaks the gate, the failure shows on that pull request (D5).
- Changed: the story's exit demonstration happens after merge, on the first manual run from `main`. Before merge, a tester runs the update script by hand (D6).
- Added: `docs/dependency-updates.md` says the dependency-merge App's credentials also open the weekly tool-update pull request (D3).

Confirmed by user: no
