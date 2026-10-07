## 1. Archive the unreleased v0 work

- [x] 1.1 Create the annotated tag `archive/unreleased-v0-main` on `git merge-base HEAD origin/main`, with the message from design D1. The message says the work was never released, that it is not the `v0.x` line, and that `v0.x` holds released v0.10.x. Ask the maintainer before pushing it, then push it. Verify that `git cat-file -t archive/unreleased-v0-main` prints `tag`, and that `git ls-remote --tags origin archive/unreleased-v0-main` prints one line.

## 2. Remove v0 from main

- [x] 2.1 Remove the v0 tree with `git rm -r`: `apps/`, `packages/`, `tools/`, `tests/`, `docker/`, `assets/`, the root `Dockerfile`, `docker-compose.yml`, `docker-compose.prod.yml`, `docker-compose.postgres.yml`, `package.json`, `package-lock.json`, `vite.config.ts`, `tailwind.config.ts`, `postcss.config.js`, `drizzle.config.ts`, `tsconfig.json`, `playwright.config.ts`, `components.json`, `test-config.js`, `.env.example`, `.env.worker.example`, `.dockerignore`, and `.trivyignore`. Keep `.nvmrc` (design D8). Verify that `git ls-files package.json package-lock.json apps packages tools tests docker assets` prints nothing.
- [x] 2.2 Remove the v0-only lines from `.gitignore`, such as SQLite database files and v0 build output. Keep the lines that `server/` and `web/` need. Verify that `git status --ignored --short server web` lists no tracked file as ignored.
- [ ] 2.3 Run `just check` inside the development shell. Verify that it exits with status 0, which shows that nothing in `server/` or `web/` depended on the removed files.

## 3. Workflows

- [x] 3.1 Delete the v0 workflows `ci.yml`, `docker-build-push.yml`, `docker-build-test.yml`, and `release.yml`. Verify the scenario "The v0.10.x workflows are gone" in the ci delta spec.
- [x] 3.2 Rename `v2.yml` to `ci.yml` with `git mv`. Set `name: ci`, change its own path in the trigger list, and add a `push` trigger for `main` with the same path filter. Keep `permissions: contents: read`. Verify every scenario of "Every CI job runs one recipe inside the development shell" and "Each workflow grants only the token access it needs" with `nix run nixpkgs#yq-go -- '<expression>' .github/workflows/ci.yml`.
- [x] 3.3 Rewrite `codeql.yml` as design D3 describes: a language matrix of `rust` and `javascript-typescript`, `build-mode: none`, branch filters for `main` only, and `contents: read` with `security-events: write`. Verify the scenarios "The CodeQL workflow has no path filter" and "The CodeQL workflow targets main only", and the permissions scenarios.
- [x] 3.4 Add `v0-security-scan.yml` as design D10 describes. Pin `aquasecurity/trivy-action` to the full commit SHA of its latest release, found with `gh api`. Verify every yq scenario of "A weekly scan checks v0.x for known vulnerabilities", plus "No workflow uses a secret".
- [x] 3.5 Run `nix run nixpkgs#actionlint` on `.github/workflows/`. Verify that it reports no error.

## 4. Dependabot

- [ ] 4.1 Edit `.github/dependabot.yml` as design D4 describes. Add `target-branch: v0.x` to the root `npm`, `docker`, and `github-actions` entries. Add a second `github-actions` entry for `main` with no `target-branch`. Remove "v2" from the comments. Verify the scenarios of "Dependabot keeps the v0.x branch up to date" and "Dependabot proposes web package updates every week" with yq.
- [ ] 4.2 Validate the file with `uv run --with check-jsonschema check-jsonschema --builtin-schema vendor.dependabot .github/dependabot.yml`. Verify that it exits with status 0.

## 5. Development commands

- [ ] 5.1 Remove the `v0-dev` and `v0-frontend` recipes and every `[group(...)]` attribute from the `justfile`. Rewrite its header comment for one stack. Verify that `just v0-dev` exits with a non-zero status, and that `just --dump --dump-format json` shows no recipe in group `v2` or `v0.10.x`.

## 6. Documents and naming

- [ ] 6.1 Rewrite `README.md` for `main`. Open it with the warning that `main` cannot be installed yet, linking to the `v0.x` branch. Then describe Sidereal briefly and link to `CONTRIBUTING.md`. Verify that it has no v0 install steps and no link into `apps/` or `assets/`.
- [ ] 6.2 Rewrite `AGENTS.md` for one stack. Keep one short section on `v0.x`: v0 fixes land there, `main` takes no v0 code, and a bug in both lines is fixed separately in each. Say that `v0.x` alerts sit under the `v0.x` branch filter in the Security tab. Move the v0 release steps out, because `main` does not release yet. Verify that `CLAUDE.md` and `GEMINI.md` still link to it.
- [ ] 6.3 Remove the v0 sections from `CONTRIBUTING.md`, and point v0 contributors to `v0.x`. Keep the `.nvmrc` instructions. Rename the `v2` workflow to `ci`. Verify that it names no `npm run` command.
- [ ] 6.4 Add a note at the top of `CHANGELOG.md` that v0.10.x entries continue on the `v0.x` branch. Verify that the existing entries are unchanged.
- [ ] 6.5 Remove "v2" as the name of `main`'s stack from `server/README.md`, `web/README.md`, `DESIGN.md`, `docs/architecture.md`, `docs/plugins.md`, `openspec/discovery.md`, `openspec/migration.md`, `openspec/config.yaml`, and code comments under `server/` and `web/`. Keep "v2" where it names a version, such as `v2.0.0`. Remove `server/README.md`'s mention of `apps/` and `packages/`. Verify with `git grep -nw v2 -- . ':!openspec/changes/archive' ':!docs/decisions'` that every match names a version or this change.
- [ ] 6.6 Edit the living specs' wording directly, as design D6 describes. Rewrite the Purpose of `ci`, `dev-commands`, and `web-shell` for one stack. Change "v2 server" to "the server" in web-shell requirements that this change's deltas do not touch. Do not edit any requirement a delta removes, modifies, or renames. Verify with `openspec validate remove-v0-from-main --strict`.

## 7. Verification

- [ ] 7.1 Run `just check`, `nix flake check`, and `openspec validate --all --strict`. Verify that each exits with status 0.
- [ ] 7.2 Push the branch. Verify on PR #352 that the `ci` workflow's `server` and `web` jobs pass, that CodeQL reports an analysis for Rust and one for JavaScript and TypeScript, and that the flake check passes.
- [ ] 7.3 Re-check the archive tag (design D1). Run `git diff --stat archive/unreleased-v0-main "$(git merge-base HEAD origin/main)" -- apps packages tools tests docker assets`. Verify that it prints nothing. If it prints a change, ask the maintainer before moving the tag.

## 8. After merge

- [ ] 8.1 Add "Closes #320" and an "After merge" checklist to PR #352's description. Verify that the checklist names these steps:
  1. delete the `ghcr.io/sidereal-io/sidereal:main` image, and check that the tag is gone;
  2. check that GitHub's notification settings send failed workflow runs to the maintainer;
  3. run `v0-security-scan.yml` by hand, and check the scenarios "A manual run uploads results for the v0.x commit it scanned" and "Alerts point at files on v0.x". If the run cannot upload, fix the workflow within a week, or revert the merge;
  4. confirm that Dependabot's next v0 version updates open against `v0.x`;
  5. close #322 and #323 as obsolete, and retitle #321 as a `v0.x` issue.
