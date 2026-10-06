## Why

`main` holds two stacks: the Rust and React rewrite, and a copy of the v0.10.x TypeScript app. The running v0 line already lives on the `v0.x` branch. Its copy on `main` adds about 265 files, six workflows, and extra Dependabot entries that new work never touches. Removing it makes `main` the one place where development happens, and lets us drop the "v2" label.

## What Changes

**Archive, then remove v0 from `main`**

- Create an annotated tag, `archive/unreleased-v0-main`, on the commit the removal builds on. `main` holds 60 commits of v0 work that no release includes, such as source plugins, local image storage, and dropping `immich_id`. The tag keeps that work easy to find. Its message states that the work was never released and that `v0.x` is the running v0 line.
- **BREAKING** for anyone running v0 from `main`: remove `apps/`, `packages/`, `tools/`, `tests/`, `docker/`, and `assets/` from `main`. Also remove the root `Dockerfile`, the compose files, `package.json` and `package-lock.json`, and the v0 root configs. The v0 root configs are `vite`, `tailwind`, `postcss`, `drizzle`, `tsconfig`, `playwright`, `components.json`, and `test-config.js`. The env examples, the root `.dockerignore`, and `.trivyignore` go too. Remove the v0-only lines from `.gitignore`.

**CI and dependency updates**

- Remove the v0 workflows from `main`: `ci.yml`, `docker-build-push.yml`, `docker-build-test.yml`, and `release.yml`. `v0.x` keeps its own copies, and GitHub runs those for `v0.x` pushes, pull requests, and tags.
- Rename `v2.yml` to `ci.yml`. It also runs on pushes to `main`, which resolves #320.
- Point CodeQL at what `main` holds: Rust and TypeScript. Drop its `v0.x` branch filter, because `v0.x` runs CodeQL from its own `ci.yml`.
- Keep `prune-ghcr.yml` on `main`, because GitHub runs scheduled workflows from the default branch only.
- Keep the v0 Dependabot entries for npm, Docker, and GitHub Actions in `main`'s config, and aim them at `v0.x` with `target-branch`. GitHub reads Dependabot's config from the default branch only.

**Container images**

- v0 keeps the `ghcr.io/sidereal-io/sidereal` tags it has today, including `:latest`, until cutover.
- Future releases from `main` use the same image with different tags. A later change defines them.
- After the removal, the maintainer deletes the stale `:main` image by hand. It was built from v0 code that no release includes.

**Names and documents**

- Stop calling `main` "v2" in living files. "Sidereal" means `main`, and "v0.10.x" or "the `v0.x` line" means the old app. Living files are the `justfile` recipe groups, the workflows, the living specs, `AGENTS.md`, `CONTRIBUTING.md`, the READMEs, `DESIGN.md`, and `docs/`.
- Leave history as written: archived changes, Accepted ADRs, and the titles of the RFC and epics.
- Open `README.md` with a warning that `main` cannot be installed yet, and point to the `v0.x` branch.
- Keep `CHANGELOG.md`, with a note that v0.10.x entries continue on `v0.x`.
- Rewrite `AGENTS.md` for one stack. It states that v0 fixes go to `v0.x` only and are never ported to `main`. The v0 release steps move to `v0.x`'s own docs.

**Issues**

- Close #322 and #323 as obsolete. Retitle #321 as a `v0.x` issue. Resolve #320 here.

**Not in scope**

- Changing the `server/` and `web/` layout. That belongs to epic #298.
- Defining how `main` releases, and which image tags it uses.
- Any change to the `v0.x` branch.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `ci`: remove the requirement "The v0.10.x pipeline skips v2-only changes". Rename the workflow file in every requirement from `v2.yml` to `ci.yml`, and run it on pushes to `main`. Limit code scanning to `main`. Drop the scenarios about pull requests that change only v0.10.x code.
- `dev-commands`: remove the requirement "The v0.10.x stack runs by explicit name". Rewrite "Recipes are grouped by stack", because only one stack remains. Rename `just dev` and the other recipes' "v2" wording.
- `web-shell`: remove the scenarios that check the web shell against the v0.10.x tree, because that tree leaves `main`. Rename "v2 server" to "the server".
- `dev-environment`: rewrite "Every reference to the Node version agrees" without the v0 production image and the v0 CI workflows. Rename the `v2` workflow in the web package update requirement.

## Impact

- **Repository:** about 265 tracked files leave `main`. Nothing under `server/` or `web/` depends on them. Only `server/README.md` names `apps/` and `packages/`.
- **CI:** four workflows leave `main`. One is renamed. Branch rules need no change, because the ruleset requires no named status checks.
- **Dependabot:** pull requests for v0 dependencies open against `v0.x` instead of `main`.
- **Images:** pushes to `main` stop publishing the v0 image. Released v0 tags do not change.
- **Contributors and agents:** `AGENTS.md`, and its links `CLAUDE.md` and `GEMINI.md`, describe one stack. `npm run check` stops being a gate on `main`.
- **Importer (ADR-010):** the importer must read the released v0 schema, which ends at migration `0008`. It must not read `main`'s unreleased `0011`.
