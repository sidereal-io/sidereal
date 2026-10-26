# Sidereal — Agent Guide

Guidance for AI agents working in this repo. `CLAUDE.md` is a symlink to this file.

## What this is

**Sidereal** is a self-hosted photo gallery and management system for astrophotographers. It provides plate solving (via Astrometry.net), equipment tracking, and deep-sky imaging metadata management.

**Stack:** Rust server (axum, cargo workspace in `server/`) · React 19 + Vite + TypeScript web interface (pnpm, in `web/`) · PostgreSQL · orchestrated by the root `justfile`

## The v0.x maintenance branch

The released app, v0.10.x (TypeScript, Hono, Drizzle), lives only on the `v0.x` branch. It stays in maintenance until cutover ([ADR-010](docs/decisions/ADR-010-migration-strategy.md)).

- **v0 fixes land on `v0.x`, never on `main`.** Branch from `v0.x` and target it. `main` takes no v0 code. The two lines share no code, so a bug in both is fixed separately in each.
- **`v0.x` has its own guide.** Its `AGENTS.md` covers its toolchain, its `npm run check` gate, and its tag-driven releases.
- **`main` watches `v0.x` in two ways,** because GitHub runs these from the default branch only. `.github/dependabot.yml` sends v0 version updates to `v0.x`. The weekly `v0-security-scan.yml` workflow scans `v0.x` for known vulnerabilities. Its alerts sit in the Security tab under the `v0.x` branch filter, not the default view.
- **The `archive/unreleased-v0-main` tag** holds v0 work that `main` carried but never released. It is not the `v0.x` line. Don't build on it.

## Durable constraints (server)

Invariants for the `server/` Rust workspace — honor them in every server change.

- **Dependency direction is one-way.** `plugin-abi` holds the public plugin contracts;
  `core` is the domain-agnostic engine that builds on `plugin-abi` and knows nothing
  about astronomy; `packs/astro` is a first-party pack that depends on `plugin-abi`
  **only, never `core`**; `server` is a thin axum binary that wires `core` + packs.
  `server/scripts/check-arch.sh` enforces this and fails the build on a violation
  ([ADR-001](docs/decisions/ADR-001-plugin-boundary.md),
  [ADR-002](docs/decisions/ADR-002-core-domain-pack-split.md)).
- **Domain logic lives in packs, never in `core`.** Astronomy — plate solving,
  equipment, deep-sky metadata — belongs in `packs/astro`, so `core` stays reusable by
  other packs.
- **`plugin-abi` is the third-party contract** — treat changes to it as public API. It
  is intentionally unfrozen and expected to churn until M2; stabilize it deliberately,
  not by accident.
- **PostgreSQL only** — no SQLite fallback on the Rust side
  ([ADR-004](docs/decisions/ADR-004-database-engine-and-schema.md)).
- **Rust follows the latest stable release.** `server/rust-toolchain.toml` names the
  `stable` channel, and `flake.lock` decides the exact release in the Nix shell and
  CI. Never put a Rust release number in a build file or doc. **`just check`**
  is the gate — green before every PR. It runs `check-compose` (Compose config checks
  with no daemon), then `check-server` (`cargo fmt --check` +
  `clippy -D warnings` + `cargo test` + arch lint), then `check-web` (type check +
  lint + format check + `DESIGN.md` lint + token drift check + unit tests for
  `web/`).
- **Real forks become ADRs** in `docs/decisions/` (template `ADR-000`). Don't design
  past a **Proposed** ADR — get it Accepted first. Each ADR stands alone: it links to
  at most one other ADR and never references issues, milestones, or the RFC.

## Durable constraints (web UI)

How screens in `web/` look is defined in [`DESIGN.md`](DESIGN.md)
([ADR-005](docs/decisions/ADR-005-visual-design-system.md)).

- **`DESIGN.md` is the source of truth** for every token value and usage rule.
  Code and the Penpot library follow it.
- **Use tokens, never raw values.** Add a semantic or component token only when a
  component first needs it, in the same pull request.
- **Change a token in `DESIGN.md` first.** Run `pnpm tokens` in `web/` to
  regenerate `tokens.css` in the same pull request. Penpot gets the change when
  an agent next copies the tokens into it. Never change a token in Penpot first.

<!-- BEGIN lets-spec rules. lets sync rewrites this block: change lets-spec, not this text. -->
## Workflow

These rules come from lets-spec, and apply to every agent.

- **Plan product changes with OpenSpec.** Work in progress lives in
  `openspec/changes/`, specs in `openspec/specs/`, and decision records
  in `docs/decisions/`. A change starts with a brainstorm and ends with
  archive. The schema's instructions say what to do at each step in
  between. [Routing](#routing) says where each request starts.
- **One change is one branch and one pull request,** from propose to
  archive. Independent changes branch off the repository's default
  branch. A change that
  depends on another stacks: it branches off the other change's branch
  and targets its pull request.
- **One commit per unit of work,** with Conventional Commits: `feat:`,
  `fix:`, `docs:`, `refactor:`, `test:`, `chore:`.
- **Run the project's checks** before you push, and fix what they
  report. The project's own instructions say how to run them.
- **When you review code, also apply the project's `REVIEW.md`,** if it
  has one.
- **The backlog is the GitHub Project linked to this repository.**
  `lets-shape` sets Kind, Priority, Size and Status; `lets-build`
  picks, claims and routes. Never set a Project field with `gh` outside
  those skills. Every piece of work is an issue, and its pull request
  says `Closes #<n>`. An epic is a parent issue with its stories as
  sub-issues. Dependencies are "blocked by" links on the story they
  block. Put a blocker on each story it blocks, never only on the epic:
  picking the next story cannot see a blocker on a parent issue. A
  "relates to" link never counts as a blocker.
- **The user owns the merge.** Never merge a pull request unless the
  user asks. Stacks merge bottom-up.
- **Change requests reopen an archived change.** If a ready pull request
  gets change requests, mark it as a draft again and `git revert` the
  archive commit. Make the fixes, archive again as the last commit, and
  mark it ready. A rejected pull request is closed and its branch
  deleted.
- **Never edit what lets-spec puts in the project:** this section,
  between its markers, the `lets-*` and `openspec-*` skills in
  `.agents/skills/` and the `lets-*` schema folders in
  `openspec/schemas/`. `lets sync` rewrites all of these, so an edit
  there is lost. Change lets-spec instead. Never run `openspec init` or
  `openspec update`; they write skills of their own.

### Routing

| Request | Start with |
|---|---|
| A large idea, or work that needs splitting or prioritizing | The `lets-shape` skill |
| A bug, a chore or a single story with no issue yet, or an issue with no packet | The `lets-shape` skill |
| Start the next story, or move the current branch's change on | The `lets-build` skill |
| Set up or check this repository's backlog | The `lets-setup-backlog` skill |

### Domain language

Read `.agents/skills/lets-shape/references/domain-language.md` before
shaping, planning, implementing or reviewing. The shared glossary is
`openspec/glossary.md`, unless the `context` in `openspec/config.yaml`
names another file. A spec folder may hold its own `glossary.md` for
narrower meanings. Agree a term's meaning and scope with the user before
you write it down. Keep that meaning in code names and reviews.

### Writing

- **Write prose in plain language** (ISO 24495): READMEs, decision
  records, issue bodies, `docs/`, pull request descriptions and OpenSpec
  artifacts. Put the reader first, and make each document easy to find,
  understand and act on. Code and config follow their own conventions.
- **Put scratch files in `.workspace/`** at the repo root, not in `/tmp`
  or a scratch path your tool suggests. Create it if it is missing, and
  keep it out of git. Anything worth keeping belongs in the repo or an
  issue.
<!-- END lets-spec rules -->

## Working in this repo

These rules add to the Let's Spec block above.

- **The backlog is [project 3](https://github.com/orgs/sidereal-io/projects/3).**
  `lets-shape` files issues and sets their fields; `lets-build` claims the next one.
  `openspec/discovery.md` holds the personas and journeys that `lets-shape` keeps. A
  milestone issue or an ADR holds the product intent.
- **Gate with `just check`** (`check-compose`, `check-server`, then `check-web`) after every change
  to `server/` or `web/`. CI runs the same checks in the `ci` workflow's `compose`,
  `server`, and `web` jobs, and its `lets-spec` job runs `lets sync --check`.
- **Epics describe outcomes.** Record the people served, scope, constraints, and
  exit demonstration. Identify groundwork and relevant code when exploring and
  cutting stories; do not put a "Relevant code" section in an epic.
- **Change a version only where it is set.** [`CONTRIBUTING.md`](CONTRIBUTING.md#where-versions-are-set)
  lists the one file that sets each tool version and product version.
- **Update Let's Spec through the flake.** Run `nix flake update lets-spec`, enter
  the dev shell so `lets sync` runs, and commit what it wrote in the same pull
  request as `flake.lock`. A Dependabot bump of `lets-spec` fails `lets sync --check`
  until someone does this on its branch.
- **`main` does not release yet.** v0.10.x releases are tagged from `v0.x`. A later
  change defines how `main` releases. Never `gh release create` manually.

## Writing document artifacts — plain language

The Let's Spec block asks for plain language (ISO 24495). In this repo, apply the core
standard (`iso-24495-1`) to all prose, and the science/technical sector standard
(`iso-24495-3`) to architecture specs, design docs, and software documentation.
