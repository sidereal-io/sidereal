<!-- Plain language: Lead with the problem and resulting behavior. Give a reader enough context to decide whether the scope solves that problem. Keep the required headings. -->

## Why

Nothing updates the tools that `mise.toml` and `mise.lock` pin. Dependabot reads neither file, so a newer Node, pnpm, `gh` or Let's Spec release reaches the repo only when Mo edits the pins, runs `mise lock` and `lets sync`, and opens a pull request by hand. Dependabot's `nix` entry can only go (#502) once the mise pins have an update route of their own.

## What Changes

- A new scheduled workflow runs every Monday at 04:00 UTC, and on demand from the Actions tab (D4). It updates the mise pins, refreshes `mise.lock`, runs `lets sync`, and opens or updates one pull request against `main` when anything changed (D5).
- The update moves every tool within the range `mise.toml` sets, so Node 26.x, pnpm 12.x and `gh` 2.x. It also moves Let's Spec to its newest release. OpenSpec stays at 2.1.1, Rust stays on the `stable` channel, and major versions of Node, pnpm and `gh` stay manual (D1).
- The workflow opens the pull request with a token from the existing dependency-merge App, so the `ci` workflow runs on it. The token asks for `contents: write` and `pull-requests: write` only (D3).
- The update and the publishing run in two jobs on separate runners. The update job holds no secret and a read-only token, and hands on only a patch. The publish job holds the App key, runs none of the updated tools, and applies the patch only when every changed path is one the update may change. A run started from a branch other than `main` does nothing and says why. These came from review of the pull request.
- The workflow does not run the gate. CI runs it on the pull request, and a failure shows there. When `mise lock` or `lets sync` fails, the run fails and opens no pull request (D5).
- The update steps live in a POSIX `sh` script under `scripts/`, so a tester can run them by hand and see what changes (D6).
- The spec stops naming one fixed Let's Spec release for `mise.lock`. A weekly update moves that release, so the scenario that says the lock records `0.8.0` would go false after the first update pull request merges. The scenario instead requires the lock to record the release `mise.toml` names. This follows from D1.
- `docs/dependency-updates.md`, `CONTRIBUTING.md` and `AGENTS.md` describe the new route and the App's second use (D3, D8).

Out of scope: merging the weekly pull request by itself, and removing Dependabot's `nix` entry (D7). `flake.nix` keeps its Let's Spec tag until #502 removes the Nix route. That change sits directly above this one in the stack, so both reach `main` before the first weekly run (D2).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `dev-environment`: adds the weekly mise tool update route and the hand-run update script. Changes the `mise.lock` requirement so its Let's Spec scenario names the release in `mise.toml` instead of `0.8.0`.
- `ci`: adds a requirement, beside "Each workflow grants only the token access it needs", that limits the new workflow to `contents: read` for its own token and two write permissions for the App token. Adds two more: only the publish job holds the App credentials, and it publishes only the allowed paths.
- `dev-environment` also gains a requirement that the weekly workflow runs only from `main`.

## Impact

- New files: a workflow under `.github/workflows/`, and POSIX `sh` scripts under `scripts/` for the update, the publish job's path check, and a fixture check of that path check.
- `mise.toml` and `mise.lock` change only through the weekly pull requests, not in this change.
- Docs: `docs/dependency-updates.md`, `CONTRIBUTING.md` (its pin table and the "To move to a newer Let's Spec" steps), and the "Update Let's Spec on both routes" rule in `AGENTS.md`.
- Credentials: reuses `DEPENDABOT_MERGE_APP_CLIENT_ID` and `DEPENDABOT_MERGE_APP_PRIVATE_KEY`. No new App and no administrator setup.
- Limit: the workflow can run only from `main`, so the exit demonstration is its first manual run after merge (D6).
