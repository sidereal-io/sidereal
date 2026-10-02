## 1. Dependabot entry

- [x] 1.1 Add an `npm` entry to `.github/dependabot.yml` for `directory: "/web"`, as design D1 and D2 describe. It runs weekly on Monday at 04:00. Its `groups` block is a copy of the root `npm` entry's block. It has one `ignore` rule for TypeScript major updates and no comment about that rule. Verify the three configuration scenarios in the delta spec with `nix run nixpkgs#yq-go -- '<expression>' .github/dependabot.yml`.
- [x] 1.2 Validate the whole file against the Dependabot schema with `uv run --with check-jsonschema check-jsonschema --builtin-schema vendor.dependabot .github/dependabot.yml`. Verify that it exits with status 0.

## 2. Review guidance

- [x] 2.1 Add one bullet to the "Reviewing a pin update" section of `CONTRIBUTING.md`, as design D4 describes. Verify that the bullet names `web/package.json`, `web/pnpm-lock.yaml`, and the `v2` workflow's `web` job, and that it does not mention TypeScript.

## 3. Verification after merge

- [x] 3.1 Dependabot reads its configuration only from `main`, so its checks can run only after merge. Add an "After merge" checklist to this change's pull request description. Verify that it names these four checks:
  1. open the repo's Dependabot status page (Insights, then Dependency graph, then Dependabot) and confirm that the `/web` entry shows no configuration error and its last run succeeded;
  2. after the first run that opens a `/web` pull request, confirm that the pull request matches the scenario "A web package has a newer release" and leaves `packageManager` unchanged;
  3. confirm that no open Dependabot pull request changes the major version of `typescript` in `web/package.json`;
  4. record the outcome on issue #333. If the hosted run fails on pnpm 12, record what failed there, as the design's Risks describe.
