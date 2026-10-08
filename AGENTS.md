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
  is the gate — green before every PR. It runs `check-server` (`cargo fmt --check` +
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

## Workflow

- Planning uses **OpenSpec**: in-flight work lives under `openspec/changes/`;
  durable specs under `openspec/specs/`; decision records under `docs/decisions/`. Use
  the `opsx:*` skills (propose → apply → verify → archive). The product input (a
  milestone issue such as #217, or an ADR) holds intent; `openspec/discovery.md`
  holds personas and journeys; the backlog is GitHub issues.
- **Generate OpenSpec skills with `just skills`.** Never run `openspec init` or
  `openspec update` — they read your global config, not the repo's settings. The
  `openspec-` prefix is reserved for generated skills: `just skills` deletes any
  `.agents/skills/openspec-*` folder, so never give an authored skill that prefix.
- OpenSpec changes carry product behavior. Repo maintenance goes through an ordinary
  branch and PR with Conventional Commits.
- **Change a version only where it is set.** [`CONTRIBUTING.md`](CONTRIBUTING.md#where-versions-are-set)
  lists the one file that sets each tool version and product version.
- **Gate with `just check`** (`check-server`, then `check-web`) after every change
  to `server/` or `web/`. CI runs the same recipes in the `ci` workflow's `server` and
  `web` jobs. Conventional Commits (`feat:`, `fix:`, `docs:`, `refactor:`, `test:`).
- **`main` does not release yet.** v0.10.x releases are tagged from `v0.x`. A later
  change defines how `main` releases. Never `gh release create` manually.

### OpenSpec git workflow

One branch and one pull request carry a change through its whole lifecycle —
propose, apply, verify, archive — and merge once. There is no "cross `main`
between phases" step.

- **Branch per change.** One OpenSpec change (one backlog issue) = one branch =
  one PR. Dependent stories **stack**: branch off the parent's branch and target
  its PR; independent stories branch off `main`.
- **A commit per unit of work.** Each artifact (proposal, design, specs, tasks) is
  its own `docs:` commit; each implementation task is its own commit with its real
  type (`feat:`/`fix:`/`refactor:`/`test:`); the archive is its own `chore:` commit.
- **Draft until archived.** Open the PR as a draft at propose, and assign its issue
  (`gh issue edit <n> --add-assignee @me`); an assigned open issue is in progress.
  Run propose → apply → verify → archive all on the branch; `archive` moves the
  change to `openspec/changes/archive/` and syncs delta specs into `openspec/specs/`.
  Flip the PR to ready when the archive commit lands.
- **User owns the merge.** The agent never merges a PR unless explicitly asks and 
  confirmed. Stacks merge bottom-up: parent to `main` first, then retarget and merge 
  each child.
- **If a ready PR gets change-requests,** flip it back to draft and `git revert` the
  archive commit — this restores the change under `openspec/changes/` and unwinds the
  spec sync. Make the fixes, re-archive as the last commit, and flip ready again. A
  rejected PR is just closed and its branch deleted; `main` stays clean.
- **Every story is an issue.** Its body is the story packet, and the story's PR
  says `Closes #<issue>`. An epic is a parent issue of type `Epic`, with its stories
  as sub-issues; close it once they are all closed. Dependencies are "blocked by"
  links between issues.
- **Every issue has an issue type:** `Epic`, `Story`, `Bug`, or `Chore`. A `Chore`
  is repo maintenance — CI, tooling, dependencies, the dev environment — that gives
  no persona anything new. Set the type with `gh issue create --type <type>` or
  `gh issue edit <n> --type <type>`. Don't use labels for the kind of issue.
- **Every story, bug, and chore carries one MoSCoW priority** in the organization's
  `Priority` issue field: `Must`, `Should`, `Could`, or `Wont`. A story's priority
  matches the `MoSCoW` line in its story packet; when you change one, change the
  other in the same edit. Epics and untriaged issues have no priority. Don't use
  labels for priority.
- **Read and set priority through the REST API.** `gh` has no issue-field commands
  yet, but `gh api` reaches them. To set an issue's priority:

  ```bash
  gh api --method POST 'repos/{owner}/{repo}/issues/<n>/issue-field-values' \
    -F "issue_field_values[][field_id]=$(gh api 'orgs/{owner}/issue-fields' --jq '.[] | select(.name == "Priority") | .id')" \
    -f 'issue_field_values[][value]=<Must|Should|Could|Wont>'
  ```

  To list the open backlog — one line per issue, with its type, priority,
  assignees, number of sub-issues, and number of open blockers:

  ```bash
  gh api 'repos/{owner}/{repo}/issues' -X GET -f state=open -f per_page=100 --paginate \
    --jq '.[] | select(.pull_request | not) | {number, title, type: .type.name,
      priority: ([.issue_field_values[]? | select(.issue_field_name == "Priority") | .single_select_option.name][0]),
      assignees: [.assignees[].login], subIssues: .sub_issues_summary.total,
      openBlockers: .issue_dependencies_summary.blocked_by}'
  ```

### Epic planning and issue relationships

- **Epics describe outcomes.** Record the people served, scope, constraints, and
  exit demonstration. Identify groundwork and relevant code when exploring and
  cutting stories; do not put a "Relevant code" section in an epic.
- **Use native GitHub relationships.** Parent/sub-issue links express ownership,
  blocked-by/blocking links express prerequisites, and relates-to links connect
  work without blocking it. Do not repeat these workflow rules in epic bodies.
- **Record blockers where they apply.** An epic-wide prerequisite belongs on the
  epic; a prerequisite for one story belongs on that story. Do not repeat an epic's
  blockers on every child or block unrelated stories on a partial prerequisite.
- **Check ancestor blockers.** Before selecting a story for implementation, inspect
  its blockers and follow its native parent links to check every ancestor's
  blockers. An open blocker at any level makes the story ineligible. Exploration
  may identify and resolve prerequisites before implementation is eligible.
- **Keep prerequisites current.** Record dependencies found during exploration
  before dependent implementation starts. A relates-to link does not satisfy a
  prerequisite or release gate.

## Writing document artifacts — plain language

Write every document artifact — READMEs, ADRs, GitHub issue bodies/designs, `docs/`,
PR descriptions, and other prose deliverables — to the **ISO 24495 Plain Language**
standard: reader-first, purposeful structure, findable, understandable, and
actionable. Apply the core standard (`iso-24495-1`) to all prose, and the
science/technical sector standard (`iso-24495-3`) to architecture specs, design docs,
and software documentation. This governs prose only — code, config, and test fixtures
follow the toolchain's own conventions.

## Scratch & Working Files

Temporary files — scratch notes, intermediate output, working scripts, throwaway data — go in
**`.workspace/`** at the repo root. It is gitignored (see `.gitignore`). **Use it instead of `/tmp`
or any scratchpad path your tooling suggests** — this convention overrides a harness-provided
scratchpad location. Create the directory if it isn't there (`mkdir -p .workspace`). Nothing durable
lives here; anything worth keeping belongs in the repo tree or a GitHub issue.
