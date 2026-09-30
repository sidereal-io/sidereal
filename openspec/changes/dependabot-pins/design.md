## Context

See proposal.md for why this change exists. The requirements are in specs/dev-environment/spec.md.

These facts about the repo shape the approach:

- `.github/dependabot.yml` already has weekly entries for npm, Docker, and GitHub Actions. They all run on Monday at 04:00.
- `flake.nix` has three inputs: `nixpkgs`, `rust-overlay`, and `flake-parts`. `nix/devshell.nix` already defines `checks.devshell`, so `nix flake check` builds the shell today.
- No CI workflow installs Nix. `ci.yml` checks the v0.10.x TypeScript app. `backend-rs.yml` checks the Rust workspace through rustup, and it runs on every change under `backend/**`, which includes `Cargo.lock`.
- The shell is built from `flake.nix`, `flake.lock`, the files in `nix/`, and `backend/rust-toolchain.toml`. `.envrc` watches the same files.
- The story asks for the simplest thing that works: no backwards-compatibility shims, no unnecessary defensive code, and no unnecessary deduplication.

## Goals / Non-Goals

**Goals:**

- Add two Dependabot entries and one small CI workflow, and change nothing else in CI.
- Accept Dependabot's default behaviour wherever it differs from what we asked for.

**Non-Goals:**

- Building the shell for macOS or ARM Linux in CI. The check runs on the one runner type, `ubuntu-latest`.
- Caching Nix builds between runs.
- Tidying the existing Dependabot entries, such as the `reviewers` key on the Actions entry.
- A before-and-after comparison of tool versions. The job prints the new versions; the reviewer compares.

## Decisions

### D1. Dependabot updates the pins, not a scheduled workflow

We add a `nix` entry and a `cargo` entry to the existing `.github/dependabot.yml`.

- **Why**: the repo already runs Dependabot. Its pull requests trigger CI, so the change needs no token and no secret.
- **Alternative**: `DeterminateSystems/update-flake-lock` in a scheduled workflow. A pull request it opens with the default `GITHUB_TOKEN` does not trigger CI. Making CI run needs a personal access token or a GitHub App, which is a credential to create and rotate.

### D2. Ask for grouping, and accept the default if it is ignored

Each new entry has a `groups` block with the pattern `"*"`. Dependabot documents grouping for Cargo. Its Nix announcement says it opens one pull request per input and does not mention groups.

- **Why**: asking costs one block of YAML. If grouping works, Mo gets one Nix pull request a week instead of three.
- **Alternative**: ignore `flake-parts` or `rust-overlay` to cut the noise. That hides real updates, so we reject it. We don't build any other workaround either.

### D3. A separate workflow file with a path filter

A new file, `.github/workflows/nix.yml`, holds one job named `flake-check`. It runs on `pull_request` events that change `flake.nix`, `flake.lock`, `nix/**`, `backend/rust-toolchain.toml`, or the workflow file itself.

- **Why**: `ci.yml` belongs to the v0.10.x app, and `backend-rs.yml` belongs to the Rust workspace. The flake belongs to neither. The path filter keeps the job off pull requests that cannot affect the shell.
- **Alternative**: add the job to `backend-rs.yml`. That workflow runs only on `backend/**`, so a `flake.lock` update would skip it.

### D4. The Determinate Nix installer, with no cache

The job installs Nix with `DeterminateSystems/determinate-nix-action@v3`. It adds no cache. Nix fetches prebuilt packages from `cache.nixos.org`, and `rust-overlay` downloads the prebuilt Rust toolchain.

- **Why**: `CONTRIBUTING.md` already recommends Determinate's installer, and the action enables flakes by default. The existing `github-actions` Dependabot entry keeps the action's version current.
- **Alternative**: `cachix/install-nix-action`, which needs flakes enabled by hand. A build cache such as Cachix is story #288.

### D5. The job prints versions to its log

After `nix flake check` passes, one step runs `nix develop --command sh -c 'openspec --version; node --version; just --version'`.

- **Why**: this is one command. A reviewer opens the job log to see the versions a pin update brings.
- **Alternative**: write the versions to the job summary, or comment on the pull request. A comment needs write permission, which a Dependabot pull request does not have. A summary adds formatting code for little gain.

### D6. The job has read-only permissions

The workflow sets `permissions: contents: read` and uses no secrets.

- **Why**: the job only reads the repo and builds it. Pull requests from forks and from Dependabot get the same read-only token, so the job behaves the same for every author.

### D7. Where the review guidance lives

`CONTRIBUTING.md` gains a section, "Reviewing a pin update", in its Development Environment part. It says three things:

- A green flake check means the shell builds with the new pins.
- An `openspec` version change also changes the skills every agent gets, so read the CLI's release notes.
- A Cargo pull request is checked by the existing Rust workflow.

## Risks / Trade-offs

- **Dependabot's Nix support is new (April 2026) and may misbehave** → we accept its default behaviour and do not work around it. A broken update shows up as a failing flake check.
- **`rust-overlay` changes daily, so its pull request will appear almost every week** → Mo merges it once the check is green. It is noise only if grouping fails.
- **The check builds the shell for one system only** → a pin that breaks the macOS shell still passes CI. Contributors on macOS catch it when their shell reloads.
- **A Cargo update can raise a version requirement in `Cargo.toml`, not just the lock** → the existing Rust workflow checks it, and a failure is ordinary review work.
- **`nix develop` runs the shell hook, which refreshes skills** → the hook never fails the shell, so the version step is unaffected.

## Migration Plan

- **Rollout**: merge the pull request. Dependabot reads the new entries on its next run. The workflow runs on the next pull request that touches the shell.
- **Rollback**: delete the two Dependabot entries and the workflow file.
