## Context

See proposal.md for why this change exists. The requirement is in specs/dev-environment/spec.md.

The repo today:

- `.github/dependabot.yml` has five entries. The root `npm` entry sets `labels`, `open-pull-requests-limit`, an `ignore` rule for `@types/node` 25, and three groups. The newer `nix` and `cargo` entries set only a schedule and one group.
- `web/package.json` pins pnpm through `packageManager: pnpm@12.8.2`. pnpm 12 writes `web/pnpm-lock.yaml` as two YAML documents. The first lists pnpm's own packages, and the second lists the project's packages.
- The `v2` workflow runs on every pull request that changes `web/**`. Its `web` job runs `just check-web`, which installs from the lockfile and then runs the type check, lint, format check, and tests.
- `web/` uses TypeScript 6.0.3 and typescript-eslint 8.71. The latest TypeScript release is 7.0.2. typescript-eslint supports TypeScript below 6.1 only, and it cannot load TypeScript 7 at all.

A local test run shows that Dependabot handles `web/` today. The run used the `dependabot` CLI 1.91.0 against `web/` on `main`, with the root entry's three groups. In that run, Dependabot:

- installed pnpm 12.8.2 from the `packageManager` field;
- read the project's packages from the second lockfile document;
- wrote a lockfile that still has both documents;
- listed the packages it tracks, and `pnpm` was not among them, so it left `packageManager` unchanged;
- opened one grouped pull request, which updated TypeScript from 6.0.3 to 7.0.2;
- skipped a 1-day-old release of `@eslint-react/eslint-plugin`, because it applies a 3-day minimum release age.

## Goals / Non-Goals

**Goals:**

- Add one Dependabot entry and one line of review guidance, and change nothing else.
- Accept Dependabot's default behaviour wherever it differs from what we expect.

**Non-Goals:**

- Running TypeScript 6 and 7 side by side.
- Copying the root entry's labels, pull request limit, or `@types/node` rule.
- Explaining the TypeScript rule in a comment or a document.

## Decisions

### D1. One `npm` entry for `/web` that copies the root entry's groups

The new entry uses the `npm` ecosystem with `directory: "/web"`. It runs weekly on Monday at 04:00. Its `groups` block is a copy of the root entry's block: production packages, development packages except `@types/*`, and `@types/*` packages.

- **Why**: the story asks for the root entry's grouping. Dependabot reads pnpm lockfiles through its `npm` ecosystem, so no other ecosystem fits.
- **Why no labels or pull request limit**: the `nix` and `cargo` entries set neither, and the new entry follows them. Grouping keeps the entry at three pull requests a week or fewer, which is below Dependabot's default limit of five.
- **Why not the `@types/node` rule**: the root rule blocks version 25 for the v0.10.x app. `web/` already uses `@types/node` 26, so the rule would do nothing.
- **Alternative**: copy the whole root entry. That adds three settings that have no effect on `web/`.

### D2. Ignore TypeScript major updates, with no comment

The entry has one `ignore` rule: `dependency-name: "typescript"` with `update-types: ["version-update:semver-major"]`. Neither `dependabot.yml` nor any document explains the rule.

- **Why**: the test run proposed TypeScript 7.0.2. typescript-eslint cannot load it, so the `web` job would fail. The failing update would sit in the development group, so the whole group's pull request would stay red every week. TypeScript patch and minor updates still arrive.
- **Why no comment**: the rule is current configuration, not history. When typescript-eslint supports TypeScript 7, someone deletes the rule, and no comment or document needs updating.
- **Alternative 1**: let the `web` job fail on the TypeScript 7 update. That blocks every other development update in the group.
- **Alternative 2**: ignore TypeScript minor updates, as the story first suggested. That rule would not stop TypeScript 7. Microsoft has also said that 6.0 is the last TypeScript release built on JavaScript and that no 6.1 is planned.
- **Alternative 3**: install TypeScript 7 as the compiler and keep TypeScript 6 for typescript-eslint. That is a separate decision with its own story.

### D3. Accept Dependabot's defaults for pnpm

The entry sets no versioning strategy, no cooldown, and no pnpm options.

- **Why**: the story asks us to accept default behaviour. The test run showed the defaults work: Dependabot raises the version range in `package.json` and updates the lockfile to match.
- **Side effect we accept**: Dependabot applies a 3-day minimum release age to pnpm updates. A release appears in a pull request at least 3 days after it is published.

### D4. One line of review guidance

`CONTRIBUTING.md`'s "Reviewing a pin update" section gains one bullet. It says that npm pull requests for `web/` change `web/package.json` and `web/pnpm-lock.yaml`, and that the `v2` workflow's `web` job checks them.

- **Why**: the section already has one bullet per kind of Dependabot pull request. The new bullet matches the Cargo bullet.

## Risks / Trade-offs

- **The hosted Dependabot may differ from the local test** → after merge, the implementer reads the `/web` job log on GitHub (Insights, then Dependency graph, then Dependabot). If the job fails on pnpm 12, the implementer records the failure on the story's issue. Choosing another tool is a separate decision.
- **Dependabot's pnpm 12 support is new** → two upstream issues remain open. One affects the dependency graph and one affects some security updates, but neither affected the version updates in the test run. We accept Dependabot's behaviour and build no workaround.
- **One failing update blocks its whole group** → the reviewer handles it as ordinary review work, for example by commenting `@dependabot ignore` on that dependency.
- **The TypeScript rule has no stated reason in the repo** → a contributor who deletes it early sees the `web` job fail on the next TypeScript 7 pull request. That failure explains itself.
- **TypeScript stays on 6 until someone removes the rule** → nothing reminds anyone when typescript-eslint gains TypeScript 7 support. We accept this. TypeScript 6 still receives patch updates.

## Migration Plan

- **Rollout**: merge the pull request. Dependabot reads the new entry from `main` on its next run.
- **Rollback**: delete the `/web` entry and the `CONTRIBUTING.md` line.
