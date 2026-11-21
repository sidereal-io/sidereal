<!-- Plain language: Help the next implementer find the approach, understand the choices and act on the plan. Explain necessary technical terms. Put each risk beside its mitigation and each open question beside the decision it affects. -->

## Context

- `mise.toml` names a range for pnpm (`"12"`) and `gh` (`"2"`), and exact releases for OpenSpec (`"2.1.1"`) and Let's Spec (`"0.8.0"`). Node comes from `.nvmrc` (`26`). Rust names the `stable` channel. `mise.lock` records one exact release per tool, with a URL and checksum for each platform already in the lock.
- mise 2026.10 behaves like this, checked with `--dry-run` on this branch:
  - `mise upgrade` installs the newest release within each range and updates `mise.lock`. Today it would move pnpm 12.9.0 → 12.22.0, `gh` 2.104.0 → 2.105.0 and Node 26.12.0 → 26.13.1. It leaves exact pins alone.
  - `mise upgrade --bump <tool>` moves one tool past its range and rewrites its version in `mise.toml`, keeping the precision (`"0.8.0"` becomes `"0.9.0"`). Let's Spec 0.8.0 is its newest release today.
  - `mise lock` with no `--platform` refreshes the platforms already in the lock, so the three required platforms stay covered.
- `lets sync` rewrites the skills, schemas and the rules block in `AGENTS.md` for the installed Let's Spec release. On this branch it exits 0 and prints one warning about OpenSpec 2.
- `.github/workflows/dependabot-automerge.yml` already creates a repository-scoped token with `actions/create-github-app-token`, from `vars.DEPENDABOT_MERGE_APP_CLIENT_ID` and `secrets.DEPENDABOT_MERGE_APP_PRIVATE_KEY`. Its jobs act only on pull requests whose author is `dependabot[bot]`, so they ignore the weekly pull request.
- GitHub does not start workflows for a push or pull request made with the default `GITHUB_TOKEN`. A push made with an App token does start them. That is why the weekly pull request needs the App token to get CI (D3).
- GitHub runs a scheduled or `workflow_dispatch` workflow only from the default branch, so nothing in this change runs before merge (D6).

## Goals / Non-Goals

**Goals:**

- Keep the update logic in one POSIX `sh` script, so CI and a tester run the same steps.
- Keep the workflow small: install, run the script, open the pull request.
- Give the workflow's own `GITHUB_TOKEN` read access only, and let only the pull-request step hold write access.

**Non-Goals:**

- Running the gate in the workflow (D5).
- Auto-merging the weekly pull request (D7).
- Updating `flake.nix` or `flake.lock` (D2).
- Detecting a new major version and reporting it.

## Test Seams

- **Primary test seam:** `scripts/update-mise-tools.sh` run by hand at the root of a throwaway Git worktree under `.workspace/`. The test reads `git status --porcelain`, `git diff` and `git rev-parse HEAD` there, then runs `scripts/check-mise-lock-platforms.sh`. A throwaway worktree keeps this branch's `mise.toml` and `mise.lock` untouched.
- **Existing seams reused:** the `yq` and `grep` reads of workflow files that the `ci` spec already uses, applied to `.github/workflows/mise-tool-updates.yml`; `scripts/check-mise-lock-platforms.sh`; `mise exec -- lets sync --check`.

No automated suite runs the script, and `mise run check` does not run it: it needs the network and changes files. The build step records each command and its result in `tasks.md`. The scenarios that need a run on `main` ("A tool has a newer release", "Last week's pull request is still open", "The weekly pull request runs CI") are shown after merge, by the first manual run (D6). The pull request says so.

## Scrutiny Tier & Blast Radius

- **Tier:** medium
- **Blast radius:** one new workflow, one new script, and three documents. The workflow holds a write-capable App token on a schedule. It can push to one branch and open or edit one pull request on this repository. It cannot merge, and it changes nothing on `main` by itself. The app, the server, the web code and the gate are untouched.

## Decisions

### 1. The script runs four steps in a fixed order

`scripts/update-mise-tools.sh` runs, with `set -eu`, so the first failing step stops it:

1. `mise upgrade --exclude rust` moves Node, pnpm and `gh` within their ranges. `--exclude rust` keeps rustup from updating the toolchain, which the lock does not record anyway.
2. `mise upgrade --bump github:sidereal-io/lets-spec` moves Let's Spec and rewrites its line in `mise.toml` (D1). OpenSpec is not named, and its exact pin keeps step 1 from moving it.
3. `mise lock` fills in every platform already in the lock. `mise upgrade` may record only the current platform for a new release.
4. `mise exec -- lets sync` writes the files for the new Let's Spec release.

It ends with `scripts/check-mise-lock-platforms.sh`, so a lock that lost a platform fails the run instead of reaching a pull request. Like the other scripts, it finds the repo root from its own path, so it works from any directory.

*Alternative:* `mise lock --bump`, which advances the lock without installing. Rejected: `lets sync` needs the new Let's Spec installed, and `mise upgrade` is what D1 names.

### 2. The workflow installs from the lock first

The job checks out `main`, runs `jdx/mise-action@v5` with `install_args: --locked` and `cache: false`, as `.github/workflows/mise.yml` does, then runs the script. `mise upgrade` upgrades installed tools, so installing first makes every tool a candidate. Installing Rust costs a few minutes once a week, which is acceptable.

### 3. `peter-evans/create-pull-request` opens or updates the one pull request

The last step passes the App token to `peter-evans/create-pull-request`, pinned to a full commit SHA with its version in a comment, as `dependabot-automerge.yml` pins its actions. It commits whatever the script changed to the fixed branch `automation/mise-tool-updates`, force-pushes it, and opens a pull request against `main`, or updates the open one. When the working tree is clean, it opens nothing and the run succeeds. A fixed branch is what keeps one pull request open at a time.

Settings: title `chore: update mise tools`, a commit message in the same form, `delete-branch: true` so a merged branch goes away, and a body that says the branch is rewritten each week and the gate runs in CI, pointing to the pull request diff for what moved.

*Alternative:* shell steps with `git push` and `gh pr create` / `gh pr edit`. Rejected: about twenty lines that reimplement "no changes, no pull request" and "update the open one", which the action already does.

### 4. Only the App token can write

The workflow sets `permissions: contents: read` at the top level, and no job widens it. The token step asks for `permission-contents: write` and `permission-pull-requests: write` only (D3). The App also holds `workflows: write` for Dependabot; this token does not ask for it, so the weekly branch cannot change a file under `.github/workflows/`. `lets sync` writes none there.

The token is created after the script runs, in the step just before `create-pull-request`. A failure in the script stops the job before any token exists.

### 5. Triggers and concurrency

`on.schedule` has the one cron `0 4 * * 1` (Monday 04:00 UTC), and `on.workflow_dispatch` has no inputs (D4). A `concurrency` group named for the workflow, with `cancel-in-progress: false`, stops a manual run and a scheduled run from pushing to the branch at the same time. The job has a `timeout-minutes` so a stuck download cannot hold the group.

### 6. Documents

- `docs/dependency-updates.md` gains a section on the weekly mise tool updates: what moves, what stays manual, that the dependency-merge App's credentials open the pull request, and that auto-merge does not apply to it. Its App setup steps gain one line saying the same App serves both workflows (D3).
- `CONTRIBUTING.md`: the sentence that says nothing updates `mise.lock` changes to name the weekly workflow. The "To move to a newer Let's Spec" steps say the weekly pull request does this on the mise route. Until #502 lands, someone still moves the `flake.nix` tag by hand.
- `AGENTS.md`: the "Update Let's Spec on both routes" rule says the weekly workflow moves Let's Spec in `mise.toml` and `mise.lock`.

No ADR: each choice here is cheap to reverse by editing one file.

## Risks / Trade-offs

- [The workflow cannot run before merge, so a typo in it shows only on `main`.] → The script carries all the logic and is run by hand before merge. The workflow file is checked by reading it with `yq`. The first manual run after merge is the exit demonstration, and a failed run opens no pull request.
- [A Let's Spec release that names OpenSpec 2 changes what `lets sync` warns about, and may need OpenSpec to move too.] → OpenSpec stays manual (D1). CI on the weekly pull request shows any mismatch, and Mo follows CONTRIBUTING's steps.
- [Between merge and #502, a weekly run moves Let's Spec in mise only, so the two routes differ.] → Merge order (D2): #502 sits directly above this branch, so both reach `main` together.
- [A compromised `create-pull-request` release could misuse the App token.] → Pin it to a full commit SHA. The token reaches only this repository, with two permissions, and lasts one job.
- [A force-push to `automation/mise-tool-updates` drops a commit someone pushed there by hand.] → The pull request body says the branch is rewritten each week. To fix an update, push to another branch.

## Migration & Rollback

N/A: nothing changes shape. To stop the updates, disable the `mise tool updates` workflow in the Actions tab, or delete the workflow file. Closing an open weekly pull request is safe; the next run opens a new one if anything is still outdated.
