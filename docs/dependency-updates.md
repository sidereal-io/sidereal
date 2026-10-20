# Dependency updates and required checks

Dependabot proposes updates using `.github/dependabot.yml`. Verified patch and
minor updates targeting `main` qualify for auto-merge only when every dependency
has a recognized patch or minor update type. Major and unknown updates, including
some Nix revisions, need manual review and merging.

`main` requires passing checks and an up-to-date PR branch. If neither the PR nor
`main` changes, the existing checks remain valid and merging needs no second CI
run. When `main` advances, stale PRs must incorporate its changes and pass checks
again. There is no merge queue.

The required checks are `CI`, `flake check`, and both CodeQL language checks.
Server, web, and Nix checks use the filters in `.github/ci-paths.yml`. Changes to
shared tooling run the affected checks; documentation-only PRs skip the expensive
server, web, and Nix jobs. The `CI` and `flake check` result jobs always report and
fail if path detection fails, a relevant job fails, or a relevant job is cancelled.
CodeQL continues to scan every PR targeting `main`.

After each push to `main`, automation updates stale Dependabot PRs that already
have auto-merge enabled. It merges the current base into each eligible branch
only when the branch is behind. The App token allows these updates to trigger CI.
A PR that fails checks or has conflicts stays unmerged. Human-authored PR branches
are never updated by this automation; maintainers use **Update branch** or merge
or rebase `main` themselves. A manual PR merges when its author or maintainer
chooses to merge it or explicitly enables auto-merge.

This setup covers `main`. Updates targeting `v0.x` retain that branch's checks
and manual merge process.

## Set up the GitHub App

An organization administrator completes these steps. Keep the private key out
of commits, issues, and chat.

1. Under the `sidereal-io` organization's **Settings → Developer settings →
   GitHub Apps**, create an App for dependency merging. Use the repository URL
   as its Homepage URL. Disable webhooks and leave OAuth options unset.
2. Grant repository permissions **Contents: Read and write**, **Pull requests:
   Read and write**, and **Workflows: Read and write**. Workflows permission
   allows GitHub Actions dependency updates to be merged. GitHub supplies read
   access to metadata automatically. No organization permissions are needed.
3. Install the App on **only `sidereal-io/sidereal`**.
4. Copy its **Client ID**, rather than App ID, into the repository's Actions
   variable `DEPENDABOT_MERGE_APP_CLIENT_ID`.
5. Generate a private key and upload its full PEM contents as the repository's
   Actions secret `DEPENDABOT_MERGE_APP_PRIVATE_KEY`.

The workflow reads verified Dependabot metadata on `pull_request_target` events.
It checks PR ownership and uses only trusted scripts from `main`; it never checks
out or executes PR code. Each installation token is scoped to this repository and
revoked when the job finishes. Updates made by the App to already-approved PRs
keep their existing auto-merge request; Dependabot's own pushes are checked again
for eligibility.

## Configure branch rules

The payload in `.github/main-required-checks.json` starts disabled for importing.
In **Settings → Rules → Rulesets**, configure **main required checks** with:

- Required checks: `CI`, `flake check`, `codeql (rust)`, and
  `codeql (javascript-typescript)`.
- **Require branches to be up to date before merging:** enabled.
- **Require merge queue:** disabled.
- Existing PR, conversation-resolution, force-push, and deletion protections.

Replace the individual `server` and `web` required checks with `CI` after this
workflow's `CI` result is available. Skipped component jobs must not be the only
merge gate. Import the payload only if the ruleset does not already exist:

```bash
gh api --method POST repos/sidereal-io/sidereal/rulesets \
  --input .github/main-required-checks.json
```

Keep repository **Settings → General → Pull Requests → Allow auto-merge** enabled.
Reopen an existing Dependabot PR without auto-merge enabled, or let Dependabot
update it, to run the eligibility workflow. Existing PRs are not enrolled merely
by merging this setup.

To pause automatic merging and branch refreshes, disable **Dependabot auto-merge**
in the Actions UI. Cancel any existing auto-merge requests that should stop too.
Keep branch protections active.

See GitHub's [required status check documentation](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-protected-branches/about-protected-branches#require-status-checks-before-merging)
and [Dependabot automation guide](https://docs.github.com/en/code-security/tutorials/secure-your-dependencies/automate-dependabot-with-actions).
