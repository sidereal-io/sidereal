# Dependabot auto-merge

The `Dependabot auto-merge` workflow enables GitHub auto-merge for verified patch
and minor updates targeting `main`. Every dependency in a group must qualify.
Major updates and updates without recognized semver metadata stay manual.

CI runs the server and web checks on every pull request and push to `main`,
so required checks report a result regardless of which files changed.
There is no merge queue or extra path-selection job. GitHub requires configured
checks and an up-to-date branch.
Existing passing checks remain usable while the PR and its base stay unchanged.
When `main` advances, update the PR branch and wait for its checks again.
The auto-merge workflow updates approved Dependabot branches when they are behind,
both when enabling auto-merge and after pushes to `main`. These App-authenticated
updates trigger CI. PRs without auto-merge enabled and human-authored PRs are left
alone. Conflicts require manual attention.
Human-authored PRs keep their normal manual merge process.

This setup covers `main`. The `v0.x` branch keeps its own CI and merge process.

## Set up the GitHub App

An organization administrator completes these steps. Keep the private key out
of commits, issues, and chat.

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

The workflow uses `pull_request_target` to read the base branch's workflow and
access these Actions credentials. It checks the PR author and repository,
verifies Dependabot metadata, and never checks out or executes PR code. It
creates an installation token scoped to this repository only for eligible
updates. The token is revoked when the job finishes.

## Repository settings

Enable **Allow auto-merge** in the repository settings. In the ruleset for
`main`, enable required checks and **Require branches to be up to date before
merging**. Leave **Require merge queue** disabled.

The Nix workflow still uses path filters. Require only checks that run on every
pull request; the auto-merge workflow never bypasses rules.

To pause automatic merging, disable **Dependabot auto-merge** in the Actions UI
and cancel any existing auto-merge requests that should stop.
