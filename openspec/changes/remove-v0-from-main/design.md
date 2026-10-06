## Context

See `proposal.md` for why `main` drops v0. This section covers only the facts that shape how.

- **GitHub reads some settings from the default branch only.** Dependabot's configuration and scheduled workflows come from `main`. Push, pull request, and tag workflows come from the branch or commit that triggered them.
- **`v0.x` is already self-sufficient.** It has its own `ci.yml` (with a CodeQL job), `docker-build-push.yml`, `docker-build-test.yml`, `prune-ghcr.yml`, and `release.yml`. Releases `v0.10.2` to `v0.10.5` were tagged from `v0.x`.
- **`main`'s v0 code is not `v0.x`'s.** `main` has 60 commits under `apps/`, `packages/`, `tools/`, and `tests/` that no release includes. Its schema runs to migration `0011`. Every release stops at `0008`.
- **Both lines publish one image,** `ghcr.io/sidereal-io/sidereal`. Pushes to `main` publish `:main`. Release tags publish `:X.Y.Z`, `:X.Y`, `:X`, and `:latest`. `prune-ghcr.yml` never deletes an image tagged `main`, `v0.x`, `latest`, or a version.
- **The ruleset needs no named checks.** It requires code scanning results and a pull request, so renaming a workflow or job breaks no merge rule.
- **Nothing on the rewrite side depends on v0.** `server/` and `web/` import nothing from `apps/` or `packages/`. Only `server/README.md` mentions those folders.

## Goals / Non-Goals

**Goals:**

- Remove v0 from `main` in a way that can be undone, and that leaves the unreleased work easy to find.
- Keep `v0.x` receiving Dependabot updates and image pruning, with no edits to `v0.x`.
- Leave every check on `main` green, including code scanning, which the ruleset requires.

**Non-Goals:**

- Add Dependabot Docker updates for `server/Dockerfile`. That is new coverage, not cleanup.
- Change the `server/` and `web/` layout, or move either folder to the root.
- Define how `main` releases, or which image tags it publishes.

## Decisions

### D1. Archive with an annotated tag, created before the removal

The first task creates `archive/unreleased-v0-main` on the commit the removal builds on, and pushes it. The tag message says the work was never released and that `v0.x` is the running v0 line.

- **Why a tag:** nobody commits to the snapshot. A tag cannot be moved by accident, stays out of the branch list, and triggers no workflow. Its name has no dots, so `release.yml`'s `v*.*.*` filter can never match it.
- **Why first:** if the tag waits until after merge, someone could delete v0 and forget to archive it. Tagging the base commit also means the tag holds exactly what the removal deletes.
- **Alternatives:** a branch `archive/v0-main` can be pushed to and clutters the branch list. Relying on git history alone works, but nothing points to the right commit.

### D2. Delete the v0 workflows, and rename `v2.yml` to `ci.yml`

`main` keeps four workflows: `ci.yml`, `codeql.yml`, `nix.yml`, and `prune-ghcr.yml`. `ci.yml` is the old `v2.yml` under the freed name. It gains a `push` trigger for `main` with the same path filter, which resolves #320.

- **Why deleting is safe:** `v0.x` runs its own copies for its pushes, pull requests, and tags. The `v0.x` entries in `main`'s copies never ran.
- **Why `ci.yml`:** `main` has one stack, so its checks are simply CI. The job names `server` and `web` stay the same.
- **Alternative:** keep the name `v2.yml`. That keeps the label this change removes.

### D3. CodeQL scans Rust and TypeScript on `main` only

`codeql.yml` runs one job per language with a matrix: `rust` and `javascript-typescript`. Both use `build-mode: none`, so no toolchain setup is needed. The branch filters name `main` only.

- **Why:** the ruleset requires code scanning results. Today CodeQL scans TypeScript only, so the Rust server has no scanning. Once `apps/` leaves, TypeScript means `web/`.
- **Alternative:** turn on GitHub's default setup. It would scan `v0.x` too, which already has its own CodeQL job. That would scan `v0.x` twice and move the configuration out of the repo.

### D4. `main`'s Dependabot config serves both branches

`main`'s `dependabot.yml` keeps its `npm`, `docker`, and `github-actions` entries for `/` and adds `target-branch: v0.x` to each. It adds a second `github-actions` entry with no `target-branch`, for `main`'s own workflows. The `nix`, `cargo`, and `/web` entries do not change.

- **Why:** Dependabot reads its config from the default branch only. Without these entries, `v0.x` gets no dependency updates.
- **Why the root `npm` entry keeps its groups:** the `/web` entry's spec requires the same groups as the root entry. Keeping both identical keeps that check meaningful.
- **Alternative:** drop v0 updates and patch `v0.x` by hand. That would leave security fixes waiting for someone to notice them.

### D5. `prune-ghcr.yml` stays, and stops protecting `:main`

The workflow keeps running weekly from `main`. Its protected-tag pattern drops `main`, so the stale `:main` image becomes ordinary and is pruned once five newer tagged images exist.

- **Why:** the maintainer deletes `:main` by hand after merge. The pattern change is a backstop if that step is missed.
- **Alternative:** keep `main` in the pattern. Then the stale image stays forever unless someone deletes it.

### D6. Spec deltas carry behavior, and wording edits go straight to the specs

The spec deltas cover requirements whose behavior changes. Many other requirements say "v2 server" or "v2 stack" with no behavior change. One task edits that wording directly in `openspec/specs/`, together with each spec's Purpose line.

- **Why:** a delta must copy every requirement it touches. Repeating five unchanged web-shell requirements just to change one word hides the real changes from reviewers.
- **Why remove and add, not modify:** OpenSpec cannot drop one scenario from a modified requirement. Where scenarios go away, the delta removes the old requirement and adds a replacement under a new name.

### D7. Remove the `justfile` stack groups

With one stack, the `v2` group holds every recipe that runs code. The change removes the `[group(...)]` attributes, so `just --list` shows one flat list.

- **Alternative:** regroup by purpose, such as `run` and `check`. With eight recipes, groups add little, and a later change can add them.

### D8. Keep `.nvmrc`

Contributors who work without Nix need Node 26 for `web/`. `CONTRIBUTING.md` tells them to use a version manager that reads `.nvmrc`. The Node-version requirement keeps `.nvmrc` and drops only its checks on the v0 image and v0 workflows.

### D9. Names: "Sidereal" means `main`, and "v0.10.x" means the old line

Living files stop saying "v2". Where a document contrasts the lines, it says "v0.10.x" or "the `v0.x` line". Archived changes, Accepted ADRs, and the RFC and epic titles stay as written, because they record past decisions. `AGENTS.md` keeps one short section on the `v0.x` branch: v0 fixes go there only and are never ported to `main`.

## Risks / Trade-offs

- [A contributor or agent opens a v0 fix against `main`] → `AGENTS.md`, `CONTRIBUTING.md`, and the README warning all point to `v0.x`. The fix has no files to change on `main`, so the mistake shows at once.
- [Someone pulls `:main` expecting current code] → The maintainer deletes `:main` after merge, and D5 removes it anyway if that step is missed.
- [`v0.x` misses Dependabot updates during the switch] → The `target-branch` entries land in the same commit that removes v0. Dependabot's next weekly run uses them.
- [Rust CodeQL with `build-mode: none` misses findings that a full build would catch] → Accept it for now. It adds Rust scanning where none exists. A later change can switch to a full build if results look thin.
- [The importer is written against `main`'s unreleased schema] → The archive tag's message and the proposal both say the importer targets migration `0008`, the last released schema.
- [Removing v0 also removes v0 references the rewrite still uses] → `server/` and `web/` import nothing from v0. The removal task runs `just check` and fails if anything breaks.

## Migration Plan

1. Create and push the annotated tag `archive/unreleased-v0-main` on the branch's base commit.
2. Land the removal, the workflow and Dependabot changes, the renames, and the documents as separate commits on this branch.
3. Run `just check` and `nix flake check`. Confirm that CodeQL reports both languages on the pull request.
4. After merge, the maintainer deletes the `:main` image in GHCR and confirms that Dependabot opens its next v0 pull requests against `v0.x`.
5. Close #322 and #323, and retitle #321 as a `v0.x` issue.

**Rollback:** revert the merge commit. Every removed file comes back from history. The tag can stay or be deleted; it changes nothing either way.
