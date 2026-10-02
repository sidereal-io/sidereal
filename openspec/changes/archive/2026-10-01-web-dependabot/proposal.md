## Why

Nothing updates the packages in `web/` today, so they miss security fixes until someone bumps them by hand. Dependabot already updates every other v2 pin file, but its `npm` entry covers only the repo root. `web/` will soon gain runtime libraries, so the gap grows.

## What Changes

- Dependabot opens weekly pull requests that update the packages in `web/package.json` and `web/pnpm-lock.yaml`. It groups them the same way as the root `npm` entry.
- Dependabot does not offer TypeScript major updates for `web/`. TypeScript stays on version 6, and patch and minor updates still arrive.
- `CONTRIBUTING.md` gains one line on reviewing a `web/` update.

A local run of Dependabot's npm updater showed that Dependabot handles `web/`'s pnpm 12 lockfile, so the change needs no workaround.

The pnpm release in `packageManager`, the Node and pnpm versions in the Nix shell, and a pnpm cache in CI stay out of scope.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `dev-environment`: adds a requirement for weekly Dependabot updates of the `web/` packages.

## Impact

- **Changed files**: `.github/dependabot.yml` gains one entry, and `CONTRIBUTING.md` gains one line.
- **Unchanged**: no code in `web/`, no CI workflow, and no other Dependabot entry changes.
- **CI**: the `v2` workflow already runs on every pull request that changes `web/`, so its `web` job checks each Dependabot pull request.
- **Reviewer load**: Mo receives up to three more grouped pull requests a week, one each for production packages, development packages, and `@types` packages.
- **Secrets**: none.
