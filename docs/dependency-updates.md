# Dependabot auto-merge

The `Dependabot auto-merge` workflow enables GitHub auto-merge for verified patch
and minor updates targeting `main` or `v0.x`. Every dependency in a group must qualify.
Major updates and updates without recognized semver metadata stay manual.

CI runs the server and web checks on every pull request and push to `main`,
so required checks report a result regardless of which files changed.
There is no merge queue or extra path-selection job. GitHub requires configured
checks and an up-to-date branch.
Existing passing checks remain usable while the PR and its base stay unchanged.
When the target branch advances, update the PR branch and wait for its checks again.
The auto-merge workflow updates approved Dependabot branches when they are behind,
both when enabling auto-merge and after pushes to the target branch. These App-authenticated
updates trigger CI. PRs without auto-merge enabled and human-authored PRs are left
alone. Conflicts require manual attention.
Human-authored PRs keep their normal manual merge process.

The workflow on `main` handles PR events for both branches. The workflow on
`v0.x` handles pushes to `v0.x`, refreshing approved maintenance PRs. Its PR
trigger is unused because GitHub reads `pull_request_target` workflows from the
default branch. The `v0.x` branch keeps its own CI checks.

## Weekly mise tool updates

Dependabot does not read `mise.toml` or `mise.lock`. The `mise tool updates`
workflow, `.github/workflows/mise-tool-updates.yml`, updates them instead. It
runs every Monday at 04:00 UTC, and when someone starts it from the Actions tab.
It works only from `main`. A manual run started from a branch other than `main`
fails at its first step, says why, and changes nothing.

The workflow runs in two jobs, on two separate runners:

1. The **update job** holds no secret, and its token can only read. It checks
   out `main`, installs the tools from `mise.lock`, and runs
   `scripts/update-mise-tools.sh`. It saves what changed as a patch file and
   hands only that file on.
2. The **publish job** starts after the update job succeeds. Only this job
   holds the App's private key. It installs and runs none of the updated
   tools. It runs `scripts/apply-mise-tool-update.sh`, which applies the
   patch only when every file in it is one an update may change:
   - `mise.toml` and `mise.lock`;
   - `AGENTS.md`, and the files under `.agents/skills/` and
     `openspec/schemas/`, which `lets sync` writes.

   The patch must also rename or copy no file, and create no link. When any
   file fails this check, the publish job names it, applies nothing and opens
   no pull request. If a new Let's Spec release writes a new kind of file,
   read its release notes, then add the path to the script in a reviewed pull
   request.

The update script changes these pins:

- Node, pnpm and `gh` move to their newest release within the major version
  that `.nvmrc` or `mise.toml` names.
- Let's Spec moves to its newest release. The script changes its version in
  `mise.toml`, then runs `lets sync` to write the skills, schemas and
  `AGENTS.md` rules block for that release.

These stay manual: OpenSpec, which `mise.toml` pins on its own; Rust, which
follows the `stable` channel; and major versions of Node, pnpm and `gh`. The
script leaves `.nvmrc` and `server/rust-toolchain.toml` alone.

When the script changes any file, the publish job commits it to the branch
`automation/mise-tool-updates` and opens a pull request against `main`. If
that pull request is still open, the run updates it instead, so only one is
open at a time. When nothing changed, the run opens nothing. The workflow
rewrites that branch each week: to fix an update, push to another branch.

The workflow does not run the gate. CI runs it on the pull request, so a
failing check there means an update broke something. When `mise lock`,
`lets sync` or the platform check fails, the run fails and opens nothing.

The publish job opens the pull request with the dependency-merge App's
credentials (see [Set up the GitHub App](#set-up-the-github-app)), because
GitHub runs no CI on a pull request opened with the workflow's default token.
The App token asks for contents and pull request write access only, not
workflow access. The pull
request's author is the App, not Dependabot, so auto-merge does not apply to
it: review and merge it by hand.

To run the update by hand, start from a clean checkout and run, at the repo
root:

```sh
mise exec -- scripts/update-mise-tools.sh
```

The script needs the network. It changes only the working tree: it commits,
pushes and opens nothing. Read `git diff`, then run `mise run check`.

To check the publish job's file check, run `sh scripts/mise-tool-update-check.sh`.
It tries allowed and refused patches in a scratch Git repository, and needs
only `git`.

## Set up the GitHub App

An organization administrator completes these steps. Keep the private key out
of commits, issues, and chat.

The same App serves two workflows: `Dependabot auto-merge` merges Dependabot's
updates, and `mise tool updates` opens the weekly mise tool update pull request.

1. Under the `sidereal-io` organization's **Settings → Developer settings →
   GitHub Apps**, create an App for dependency merging. Disable webhooks;
   the workflow starts the App's work.
2. Grant repository permissions **Contents: Read and write**, **Pull requests:
   Read and write**, and **Workflows: Read and write**. The Workflows permission
   allows GitHub Actions dependency updates to be merged. GitHub supplies read
   access to metadata automatically.
   No organization permissions or ruleset bypass permissions are needed.
3. Install the App on **only `sidereal-io/sidereal`**.
4. Copy its Client ID into the repository's Actions variable
   `DEPENDABOT_MERGE_APP_CLIENT_ID`.
5. Generate a private key and upload its PEM contents as the repository's
   Actions secret `DEPENDABOT_MERGE_APP_PRIVATE_KEY`.

`Dependabot auto-merge` uses `pull_request_target` to read the default branch's
workflow and access these Actions credentials. It checks the PR author and
repository, verifies Dependabot metadata, and never checks out or executes PR
code. It creates an installation token scoped to this repository only for
eligible updates. The token is revoked when the job finishes.
[Weekly mise tool updates](#weekly-mise-tool-updates) isolates the App key in
its publish job, requests narrower token permissions, and never runs updated
tools on the runner that holds the key.

## Repository settings

Enable **Allow auto-merge** in the repository settings. In the rulesets for
`main` and `v0.x`, enable each branch's required checks and **Require branches
to be up to date before merging**. Leave **Require merge queue** disabled.

The `mise` workflow uses path filters, so it does not run on every pull
request. Require only checks that run on every pull request; the auto-merge
workflow never bypasses rules.

To pause automatic merging, disable **Dependabot auto-merge** in the Actions UI
and cancel any existing auto-merge requests that should stop.
