# Sidereal — Agent Guide

Guidance for AI agents working in this repo. `CLAUDE.md` is a symlink to this file.

## What this is

**Sidereal** is a self-hosted photo gallery and management system for astrophotographers. It integrates with [Immich](https://immich.app/) to provide plate solving (via Astrometry.net), equipment tracking, and deep-sky imaging metadata management.

**Stack:** TypeScript monorepo · React 19 + Vite (frontend) · Hono (backend) · Drizzle ORM · SQLite (default) / PostgreSQL · Playwright (E2E)

## Current state: v0.10.x and v2 in progress

Sidereal is mid-rewrite, so two stacks live side by side in this repo ([RFC #213](https://github.com/sidereal-io/sidereal/issues/213)). Every change belongs to exactly one of them:

| Stack | Lives in | What it is | Status |
|---|---|---|---|
| **v0.10.x** | `apps/`, `packages/` | The running TypeScript/Hono app — deployed today, and what most work still touches until cutover ([ADR-010](docs/decisions/ADR-010-migration-strategy.md)) | Current |
| **v2** | `backend/` | A new Rust backend ([ADR-009](docs/decisions/ADR-009-backend-language.md)) in a separate cargo workspace | Under active build |

The frontend stays TypeScript/React through the whole rewrite ([ADR-005](docs/decisions/ADR-005-frontend-continuity.md)); only the backend changes language.

**v2 builds in milestones.** M0 (scaffolding) is done. M1 — the core spine and first plugins ([#217](https://github.com/sidereal-io/sidereal/issues/217)) — is in `status/design`.

**Run both stacks from the root `justfile`.** It is the single front door: `just dev` starts the Rust backend and the Vite frontend together, and `just --list` describes every recipe.

**An optional, pinned Nix shell provides every tool both stacks need**, including `just` itself. `just check` works the same inside it or with each tool installed by hand. See [`CONTRIBUTING.md`](CONTRIBUTING.md#development-environment).

**Where to read more:**

- **v2 backend layout, prerequisites, and commands** — [`backend/README.md`](backend/README.md).
- **v2 target architecture and milestone plan** — [`docs/architecture/README.md`](docs/architecture/README.md) and [`roadmap.md`](docs/architecture/roadmap.md).
- **The rest of this file** — describes the v0.10.x stack, plus the v2 constraints and cross-stack workflow in the sections below.

## Durable constraints (v2 backend)

Invariants for the `backend/` Rust workspace — honor them in every v2 change.

- **Dependency direction is one-way.** `plugin-abi` holds the public plugin contracts;
  `core` is the domain-agnostic engine that builds on `plugin-abi` and knows nothing
  about astronomy; `packs/astro` is a first-party pack that depends on `plugin-abi`
  **only, never `core`**; `server` is a thin axum binary that wires `core` + packs.
  `backend/scripts/check-arch.sh` enforces this and fails the build on a violation
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
- **Toolchain is pinned.** Stable Rust 1.85 via `backend/rust-toolchain.toml`;
  **`just check`** (`cargo fmt --check` + `clippy -D warnings` + `cargo test` + arch
  lint) is the gate — green before every PR.
- **Real forks become ADRs** in `docs/decisions/` (template `ADR-000`). Don't design
  past a **Proposed** ADR — get it Accepted first. Each ADR stands alone: it links to
  at most one other ADR and never references issues, milestones, or the RFC.

## Workflow

- Planning uses **OpenSpec**: in-flight work lives under `openspec/changes/`;
  durable specs under `openspec/specs/`; decision records under `docs/decisions/`. Use
  the `opsx:*` skills (propose → apply → verify → archive). The product input (a
  milestone issue such as #217, or an ADR) holds intent; `openspec/discovery.md`
  holds personas and journeys; the backlog is GitHub issues.
- OpenSpec changes carry product behavior. Repo maintenance goes through an ordinary
  branch and PR with Conventional Commits.
- **Toolchain is per stack.** v0.10.x: Node 26, npm, Vite/React, Hono, Drizzle —
  gate with **`npm run check`** (TypeScript) after every change to `apps/`/`packages/`.
  v2: cargo workspace under `backend/`, orchestrated by the root `justfile` — gate with
  **`just check`** (fmt + clippy `-D warnings` + tests + arch-boundary lint) after every
  change to `backend/`. Conventional Commits (`feat:`, `fix:`, `docs:`, `refactor:`,
  `test:`).
- **Releases are tag-driven** — bump `package.json`, add a `CHANGELOG.md` entry, then
  push a `v*.*.*` tag; the workflow does the rest. Never `gh release create` manually.

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
  says `Closes #<issue>`. An epic is a parent issue, labeled `epic`, with its stories
  as sub-issues; close it once they are all closed. Dependencies are "blocked by"
  links between issues.

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
