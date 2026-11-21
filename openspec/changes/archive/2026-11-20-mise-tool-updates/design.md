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
- GitHub starts a scheduled or `workflow_dispatch` workflow only once its file is on the default branch, so nothing in this change runs before merge (D6). A scheduled run always uses the default branch. A manual run uses the branch the operator picks in the Actions tab, and that branch's copy of the workflow file, so the workflow checks its own branch (Decision 9).
- `lets sync` 0.8.0 changes, on a checkout that already has them, only the rules block in `AGENTS.md`, the skills in `.agents/skills/` and the schemas in `openspec/schemas/` (`lets sync --help`). It creates `openspec/config.yaml`, `CLAUDE.md` and the `.claude/skills` link only when they are missing, and edits `.gitignore` only with `LETS_SPEC_SKILLS=ignored`, which this repo does not set. It writes nothing under `.github/workflows/`. The past Let's Spec updates in `git log` (#464, #479) changed only those three places, plus a hand edit to `openspec/config.yaml` that followed the release's upgrading notes.

## Goals / Non-Goals

**Goals:**

- Keep the update logic in one POSIX `sh` script, so CI and a tester run the same steps.
- Keep the workflow small: one job runs the update, a second job publishes it.
- Give the workflow's own `GITHUB_TOKEN` read access only, and let only the publish job hold write access.
- Keep the App's private key off every runner that installs or runs a newly selected tool release.

**Non-Goals:**

- Running the gate in the workflow (D5).
- Auto-merging the weekly pull request (D7).
- Updating `flake.nix` or `flake.lock` (D2).
- Detecting a new major version and reporting it.

## Test Seams

- **Primary test seam:** `scripts/update-mise-tools.sh` run by hand at the root of a throwaway Git worktree under `.workspace/`. The test reads `git status --porcelain`, `git diff` and `git rev-parse HEAD` there, then runs `scripts/check-mise-lock-platforms.sh`. A throwaway worktree keeps this branch's `mise.toml` and `mise.lock` untouched.
- **Existing seams reused:** the `yq` and `grep` reads of workflow files that the `ci` spec already uses, applied to `.github/workflows/mise-tool-updates.yml`; `scripts/check-mise-lock-platforms.sh`; `mise exec -- lets sync --check`.

- **Publish seam:** `scripts/mise-tool-update-check.sh` builds a scratch Git repository, makes patches there the way the update job does, and runs `scripts/apply-mise-tool-update.sh` on each: an allowed change, an empty patch, and patches the publish job must refuse. It needs no network and changes no tracked file, so a tester runs it by hand.
- **Branch seam:** the update job's first step is plain shell that reads `GITHUB_REF`. A test takes its text from the workflow with `yq` and runs it with `sh`, once with `GITHUB_REF=refs/heads/main` and once with another branch.

No automated suite runs the update script, and `mise run check` does not run it: it needs the network and changes files. The build step records each command and its result in `tasks.md`. The scenarios that need a run on `main` ("A tool has a newer release", "Last week's pull request is still open", "The weekly pull request runs CI") are shown after merge, by the first manual run (D6). The pull request says so.

## Scrutiny Tier & Blast Radius

- **Tier:** medium
- **Blast radius:** one new workflow, three new scripts, and three documents. The workflow holds a write-capable App token on a schedule, in its publish job only. It can push to one branch and open or edit one pull request on this repository. It cannot merge, and it changes nothing on `main` by itself. The app, the server, the web code and the gate are untouched.

## Decisions

### 1. The script runs four steps in a fixed order

`scripts/update-mise-tools.sh` runs, with `set -eu`, so the first failing step stops it:

1. `mise upgrade --local --exclude rust` moves Node, pnpm and `gh` within their ranges. `--local` limits the upgrade to the tools this repo's config names, so a hand run leaves a contributor's global tools alone. `--exclude rust` keeps rustup from updating the toolchain, which the lock does not record anyway.
2. `mise upgrade --bump github:sidereal-io/lets-spec` moves Let's Spec and rewrites its line in `mise.toml` (D1). OpenSpec is not named, and its exact pin keeps step 1 from moving it.
3. `mise lock` fills in every platform already in the lock. `mise upgrade` may record only the current platform for a new release.
4. `mise exec -- lets sync` writes the files for the new Let's Spec release.

It ends with `scripts/check-mise-lock-platforms.sh`, so a lock that lost a platform fails the run instead of reaching a pull request. Like the other scripts, it finds the repo root from its own path, so it works from any directory.

*Alternative:* `mise lock --bump`, which advances the lock without installing. Rejected: `lets sync` needs the new Let's Spec installed, and `mise upgrade` is what D1 names.

### 2. The update job installs from the lock first

The update job checks out `${{ github.sha }}`, the commit on `main` that the run started from (Decision 9), runs `jdx/mise-action@v5` with `install_args: --locked` and `cache: false`, as `.github/workflows/mise.yml` does, then runs the script. `mise upgrade` upgrades installed tools, so installing first makes every tool a candidate. Installing Rust costs a few minutes once a week, which is acceptable.

### 3. `peter-evans/create-pull-request` opens or updates the one pull request

The publish job's last step passes the App token to `peter-evans/create-pull-request`, pinned to a full commit SHA with its version in a comment, as `dependabot-automerge.yml` pins its actions. It commits whatever the script changed to the fixed branch `automation/mise-tool-updates`, force-pushes it, and opens a pull request against `main`, or updates the open one. When the working tree is clean, it opens nothing and the run succeeds. A fixed branch is what keeps one pull request open at a time.

Settings: title `chore: update mise tools`, a commit message in the same form, `delete-branch: true` so a merged branch goes away, and a body that says the branch is rewritten each week and the gate runs in CI, pointing to the pull request diff for what moved.

*Alternative:* shell steps with `git push` and `gh pr create` / `gh pr edit`. Rejected: about twenty lines that reimplement "no changes, no pull request" and "update the open one", which the action already does.

### 4. Only the App token can write

The workflow sets `permissions: contents: read` at the top level, and no job widens it. The token step asks for `permission-contents: write` and `permission-pull-requests: write` only (D3). The App also holds `workflows: write` for Dependabot; this token does not ask for it, so the weekly branch cannot change a file under `.github/workflows/`. `lets sync` writes none there, and the publish job refuses such a path before it creates the token (Decision 8).

The token is created in the publish job, after the path check and the apply, in the step just before `create-pull-request`. A failed update skips the publish job, so no token exists.

### 5. Triggers and concurrency

`on.schedule` has the one cron `0 4 * * 1` (Monday 04:00 UTC), and `on.workflow_dispatch` has no inputs (D4). A `concurrency` group named for the workflow, with `cancel-in-progress: false`, stops a manual run and a scheduled run from pushing to the branch at the same time. Each job has a `timeout-minutes` so a stuck download cannot hold the group.

### 6. Documents

- `docs/dependency-updates.md` gains a section on the weekly mise tool updates: what moves, what stays manual, that the dependency-merge App's credentials open the pull request, and that auto-merge does not apply to it. Its App setup steps gain one line saying the same App serves both workflows (D3).
- `CONTRIBUTING.md`: the sentence that says nothing updates `mise.lock` changes to name the weekly workflow. The "To move to a newer Let's Spec" steps say the weekly pull request does this on the mise route. Until #502 lands, someone still moves the `flake.nix` tag by hand.
- `AGENTS.md`: the "Update Let's Spec on both routes" rule says the weekly workflow moves Let's Spec in `mise.toml` and `mise.lock`.
- After review: `docs/dependency-updates.md` says the update and the publishing run in two jobs, that only the publish job holds the App key, which files the publish job accepts, and that a run from another branch does nothing. `CONTRIBUTING.md` says a weekly pull request changes only those files.

No ADR: each choice here is cheap to reverse by editing one file.

### 7. Two jobs on two runners

The workflow has two jobs. Upgraded code never shares a runner with the App's private key.

- **`update`** has no secret and no App variable, and its `GITHUB_TOKEN` can only read. It refuses a run from another branch (Decision 9), checks out the run's commit, installs from the lock, and runs `scripts/update-mise-tools.sh`. It then stages every change with `git add -A` and saves `git diff --cached --binary --no-renames` as `mise-tool-update.patch`, uploaded as the artifact `mise-tool-update`. The patch is empty when nothing changed.
- **`publish`** `needs: update`, so a failed update skips it. It runs on a fresh runner, checks out the same `${{ github.sha }}`, downloads the artifact into `$RUNNER_TEMP`, outside the work tree, and runs `scripts/apply-mise-tool-update.sh` on the patch (Decision 8). Only then does it create the App token and run `create-pull-request`. It installs no tool with mise and runs no `mise` command.

Only the patch crosses between the jobs. The publish job treats it as untrusted data: it reads it only through `git apply`, passes its path as one quoted argument, and uses nothing from it as a command, a ref or a workflow expression. The publish job reads no `needs.update.outputs`. The pull request's title, body, branch and commit message are fixed in the workflow file, so the update job can change none of them.

*Alternative:* one job that creates the token after the update, as before. Rejected: a newly released tool could write to `$GITHUB_ENV` or `$GITHUB_PATH`, replace `git` or plant files that the later steps read, and so reach the private key that the token step receives.

*Alternative:* hand over the changed files as an archive. Rejected: unpacking must then guard against paths outside the work tree and links. `git apply` already refuses both, and lists exactly the paths it would touch.

### 8. The publish job applies only allowed paths

`scripts/apply-mise-tool-update.sh PATCH` checks the whole patch before it changes anything:

1. An empty patch means nothing changed: the script exits 0 and changes nothing.
2. `git apply --summary` must show no rename or copy, and every created file or new mode must be `100644` or `100755`. A link or a submodule fails the check.
3. `git apply --numstat`, the same parser that applies the patch, lists every path. Each must be on the allowed list:
   - `mise.toml` and `mise.lock`, which `mise upgrade` and `mise lock` change;
   - `AGENTS.md`, `.agents/skills/…` and `openspec/schemas/…`, which `lets sync` changes (see Context).

   A path with `..` in it, or one Git prints quoted because it holds a control character, is not on the list.
4. When every path passes, it runs `git apply` once. Git applies all of the patch or none of it.

On any refused path the script names each one on its own line, starting with `apply-mise-tool-update:` so that no line of the log starts with a path, and exits 1 before the token step. A new kind of file in a later Let's Spec release fails the publish job until someone adds it to the list in a reviewed pull request.

*Alternative:* apply first, then check `git status`. Rejected: the check must come before anything changes.

### 9. Only a run from `main` does anything

The update job's first step, before checkout, compares `GITHUB_REF` with `refs/heads/main`. On any other ref it fails with an error that names the ref and says the workflow runs only from `main`. The publish job also carries `if: github.ref == 'refs/heads/main'`. The step names `main` because `create-pull-request` already names it as `base`, and a scheduled run's event may carry no repository details to read the default branch from.

*Limit:* a manual run from another branch uses that branch's copy of the workflow file. The check stops a mistaken run from a branch that keeps it, not a branch that removed it. A GitHub environment that only `main` may use would hold the key against that. Setting one up is an administrator change, outside this change.

## Risks / Trade-offs

- [The workflow cannot run before merge, so a typo in it shows only on `main`.] → The script carries all the logic and is run by hand before merge. The workflow file is checked by reading it with `yq`. The first manual run after merge is the exit demonstration, and a failed run opens no pull request.
- [A Let's Spec release that names OpenSpec 2 changes what `lets sync` warns about, and may need OpenSpec to move too.] → OpenSpec stays manual (D1). CI on the weekly pull request shows any mismatch, and Mo follows CONTRIBUTING's steps.
- [Between merge and #502, a weekly run moves Let's Spec in mise only, so the two routes differ.] → Merge order (D2): #502 sits directly above this branch, so both reach `main` together.
- [A compromised `create-pull-request` release could misuse the App token.] → Pin it to a full commit SHA. The token reaches only this repository, with two permissions, and lasts one job.
- [A compromised tool release runs in the update job.] → It holds a read-only token and no secret. It can only shape the patch, which the publish job checks path by path before it creates the App token (Decisions 7 and 8).
- [A later Let's Spec release writes a file outside the allowed list.] → The publish job fails and names the file. Read the release notes, then add the path to `scripts/apply-mise-tool-update.sh` in a reviewed pull request.
- [The artifact step may handle an empty patch differently from a local test.] → The apply script treats an empty patch as "nothing changed". The first manual run after merge, which likely finds updates, and a later run that finds none, show the hosted behaviour.
- [A force-push to `automation/mise-tool-updates` drops a commit someone pushed there by hand.] → The pull request body says the branch is rewritten each week. To fix an update, push to another branch.

## Migration & Rollback

N/A: nothing changes shape. To stop the updates, disable the `mise tool updates` workflow in the Actions tab, or delete the workflow file. Closing an open weekly pull request is safe; the next run opens a new one if anything is still outdated.
