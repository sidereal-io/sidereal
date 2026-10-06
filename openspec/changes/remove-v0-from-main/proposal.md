## Why

`main` holds two stacks: the Rust and React rewrite, and a copy of the v0.10.x TypeScript app. The running v0 line already lives on the `v0.x` branch. Its copy on `main` adds about 265 files, six workflows, and extra Dependabot entries that new work never touches. Removing it makes `main` the one place where new development happens, and lets us drop "v2" as the name of `main`'s stack.

## What Changes

**Archive, then remove v0 from `main`**

- Create an annotated tag, `archive/unreleased-v0-main`, on the commit the removal builds on. `main` holds 60 commits of v0 work that no release includes, such as source plugins, local image storage, and dropping `immich_id`. The tag keeps that work easy to find. Its message states that the work was never released and that `v0.x` is the running v0 line.
- **BREAKING** for anyone running v0 from `main`: remove `apps/`, `packages/`, `tools/`, `tests/`, `docker/`, and `assets/` from `main`. Also remove the root `Dockerfile`, the compose files, `package.json` and `package-lock.json`, and the v0 root configs. The v0 root configs are `vite`, `tailwind`, `postcss`, `drizzle`, `tsconfig`, `playwright`, `components.json`, and `test-config.js`. The env examples, the root `.dockerignore`, and `.trivyignore` go too. Remove the v0-only lines from `.gitignore`.

**CI and dependency updates**

- Remove the v0 workflows from `main`: `ci.yml`, `docker-build-push.yml`, `docker-build-test.yml`, and `release.yml`. `v0.x` keeps its own copies, and GitHub runs those for `v0.x` pushes, pull requests, and tags.
- Rename `v2.yml` to `ci.yml`. It also runs on pushes to `main`, which resolves #320.
- Point CodeQL at what `main` holds: Rust and TypeScript. Drop its `v0.x` branch filter, because `v0.x` runs CodeQL from its own `ci.yml`.
- Keep `prune-ghcr.yml` on `main`, because GitHub runs scheduled workflows from the default branch only.
- Keep the v0 Dependabot entries for npm, Docker, and GitHub Actions in `main`'s config, and aim them at `v0.x` with `target-branch`. GitHub reads Dependabot's config from the default branch only. These entries cover version updates only.
- Add a weekly workflow on `main` that scans `v0.x`'s dependency manifests for known vulnerabilities. Dependabot alerts and security updates read the default branch only, so without this scan nothing would warn about a new advisory on `v0.x`. The run fails on a high or critical finding, so the failure shows in the maintainer's workflow notifications.
- Grant each workflow this change touches only the token access it needs.

**Container images**

- v0 keeps the `ghcr.io/sidereal-io/sidereal` tags it has today, including `:latest`, until cutover.
- Future releases from `main` use the same image with different tags. A later change defines them.
- After the removal, the maintainer deletes the stale `:main` image by hand, and checks that it is gone. It was built from v0 code that no release includes, and `prune-ghcr.yml` never deletes it on its own.

**Names and documents**

- Stop using "v2" as the name of `main`'s stack in living files. "Sidereal" means `main`, and "v0.10.x" or "the `v0.x` line" means the old app. Living files are the `justfile` recipe groups, the workflows, the living specs, `AGENTS.md`, `CONTRIBUTING.md`, the READMEs, `DESIGN.md`, `docs/`, and `openspec/`.
- Keep "v2" where it names a version. The cutover release is still planned as `v2.0.0`.
- Leave history as written: archived changes, Accepted ADRs, and the titles of the RFC and epics.
- Open `README.md` with a warning that `main` cannot be installed yet, and point to the `v0.x` branch.
- Keep `CHANGELOG.md`, with a note that v0.10.x entries continue on `v0.x`.
- Rewrite `AGENTS.md` for one stack. It states that v0 fixes land on `v0.x`, and that `main` takes no v0 code. A bug that exists in both lines is fixed separately in each. The v0 release steps move to `v0.x`'s own docs.

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

- `ci`: remove the requirement "The v0.10.x pipeline skips v2-only changes". Rename the workflow file from `v2.yml` to `ci.yml`, and run it on pushes to `main`. Limit code scanning to `main`, and add Rust. Require least-privilege tokens. Add the weekly `v0.x` vulnerability scan. Require that no workflow on `main` builds images or publishes releases. Drop the scenarios about pull requests that change only v0.10.x code.
- `dev-commands`: remove the requirements "The v0.10.x stack runs by explicit name" and "Recipes are grouped by stack". Rename `just dev`'s requirement so it no longer says "v2".
- `web-shell`: replace "The web shell installs and runs on its own" so it no longer checks the v0.10.x root npm files, which leave `main`.
- `dev-environment`: replace "Every reference to the Node version agrees" without the v0 production image. Rename the `v2` workflow in the web package update requirement. Add the Dependabot entries that target `v0.x`.

## Impact

- **Repository:** about 265 tracked files leave `main`. Nothing under `server/` or `web/` depends on them. Only `server/README.md` names `apps/` and `packages/`.
- **CI:** four workflows leave `main`, one is renamed, and one is added. The branch ruleset is disabled today, and it requires no named status checks, so no merge rule breaks.
- **Dependabot:** version update pull requests for v0 dependencies open against `v0.x` instead of `main`. v0 security warnings come from the weekly scan instead of Dependabot alerts.
- **Images:** pushes to `main` stop publishing the v0 image. Released v0 tags do not change.
- **Contributors and agents:** `AGENTS.md`, and its links `CLAUDE.md` and `GEMINI.md`, describe one stack. `npm run check` stops being a gate on `main`.
- **Importer (ADR-010):** the importer must read the schema of the latest v0 release at cutover. Today that schema ends at migration `0008`. The importer must not read `main`'s unreleased `0011`.
