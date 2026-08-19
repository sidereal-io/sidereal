# Sidereal — Agent Guide

**Sidereal** is a self-hosted photo gallery and management system for astrophotographers. It integrates with [Immich](https://immich.app/) to provide plate solving (via Astrometry.net), equipment tracking, and deep-sky imaging metadata management.

**Stack:** TypeScript monorepo · React 19 + Vite (frontend) · Hono (backend) · Drizzle ORM · SQLite (default) / PostgreSQL · Playwright (E2E)

## Repository Structure

```
apps/
  client/          # React 19 frontend (Vite, Tailwind CSS 4, shadcn/ui, TanStack Query, Wouter)
  server/          # Hono backend
    src/
      routes/      # API route handlers (one file per resource)
      services/    # Business logic (astrometry, catalog, config, cron, websocket, worker)
      workers/     # Background job processors
      db.ts        # Database connection
packages/
  shared/          # Code shared between client and server
    src/
      db/          # Drizzle schemas — sqlite-schema.ts and postgres-schema.ts
      schemas/     # Zod validation schemas
      types/       # TypeScript type definitions
      utils/       # Shared utilities
tools/
  migrations/      # Drizzle-generated SQLite migrations
  scripts/         # Build, seed, and DB migration helper scripts
tests/
  e2e/             # Playwright E2E tests (Page Object Model in tests/e2e/pages/)
docker/            # Dockerfile, docker-compose files, startup.sh
.github/workflows/ # CI/CD pipelines
```

## Prerequisites

- Node.js 20+
- npm (comes with Node)
- Docker (optional — only needed for container workflows)

No external database required for local development; SQLite is used by default.

## Setup

```bash
npm install
```

Copy the example env file before running locally:

```bash
cp .env.example .env.local
```

## Key Commands

| Command | What it does |
|---|---|
| `npm run dev` | Start full stack: server + frontend + worker |
| `npm run dev:server` | Backend server only |
| `npm run dev:frontend` | Vite dev server only |
| `npm run dev:worker` | Worker process only |
| `npm run check` | TypeScript type checking — **run this after every change** |
| `npm run test` | Unit tests (`packages/shared/src/**/*.test.ts`) |
| `npm run test:e2e` | Playwright E2E tests (requires app on port 5173) |
| `npm run build` | Production build (frontend + backend) |
| `npm run db:migrate` | Apply Drizzle migrations |
| `npm run docker:build` | Build Docker image |
| `npm run docker:run` | Start via docker-compose |

## Architecture

### Backend

- **Framework:** Hono — lightweight, edge-compatible web framework
- **Pattern:** One route file per resource in `apps/server/src/routes/`. Business logic lives in `apps/server/src/services/`. Long-running work goes in `apps/server/src/workers/`.
- **Adding a new endpoint:** Create a route file, register it in the server entry point, add shared types/schemas in `packages/shared/`.

### Database

- **ORM:** Drizzle — type-safe query builder
- **Default:** SQLite (`local.db` locally, `/app/config/sidereal.db` in Docker)
- **Optional:** PostgreSQL — set `DATABASE_URL=postgresql://...`
- **Schemas:** `packages/shared/src/db/sqlite-schema.ts` and `postgres-schema.ts`
- **Migrations:** Generated to `tools/migrations/sqlite/` and applied automatically on startup

### Frontend

- **Data fetching:** TanStack React Query
- **Routing:** Wouter
- **Real-time updates:** WebSocket
- **UI components:** shadcn/ui — prefer these over writing custom components
- **Styling:** Tailwind CSS 4

### Validation

- **Library:** Zod — always validate at API boundaries
- **Location:** `packages/shared/src/schemas/`

## Code Conventions

- **Modules:** ES modules (`import`/`export`) throughout — no CommonJS
- **TypeScript:** Strict mode; avoid `any`
- **Commits:** Conventional Commits — `feat:`, `fix:`, `docs:`, `refactor:`, `test:`
- **UI:** Use shadcn/ui components when available; follow existing Tailwind patterns
- **Linting:** Follow the existing style in surrounding code

## Scratch & Working Files

Temporary files — scratch notes, intermediate output, working scripts, throwaway data — go in
**`.workspace/`** at the repo root. It is gitignored (see `.gitignore`). **Use it instead of `/tmp`
or any scratchpad path your tooling suggests** — this convention overrides a harness-provided
scratchpad location. Create the directory if it isn't there (`mkdir -p .workspace`). Nothing durable
lives here; anything worth keeping belongs in the repo tree or a GitHub issue.

## Environment Variables

| Variable | Default | Description |
|---|---|---|
| `PORT` | `5000` | Server port |
| `NODE_ENV` | `development` | Environment |
| `DATABASE_URL` | *(unset)* | PostgreSQL connection string; omit to use SQLite |
| `STORAGE_PATH` | `./data/images` (`/app/data/images` in container) | Root directory for local image storage (`processed/` subtree) |

Runtime configuration (Immich API key, Astrometry.net API key, sync schedules, plate-solving settings) is managed through the admin UI at `/admin` and persisted in the database — it takes precedence over environment variables.

See `.env.example` and `.env.worker.example` for the full list.

## Testing

- Run `npm run check` after every change — this is the minimum bar.
- Unit tests live alongside source in `packages/shared/src/`.
- E2E tests use the Page Object Model; page objects are in `tests/e2e/pages/`.
- E2E tests require the dev server running (`npm run dev`) before invoking `npm run test:e2e`.

## CI/CD

- **Every PR:** TypeScript check + Docker build (AMD64 only)
- **Merge to `main`:** Multi-arch Docker build (amd64 + arm64) pushed to GHCR
- **Semver tag (`v*.*.*`):** Full release — multi-arch image, SBOM, GitHub release with artifacts

## Decision Records

When you encounter a design decision with multiple valid approaches, suggest a decision record. They're most useful when a reasonable person could argue for a different approach.

New ADRs follow `docs/decisions/ADR-000-template.md`. Number sequentially (`ADR-NNN-short-slug.md`). Keep options concrete — include code snippets or config examples where they clarify trade-offs.

### Lifecycle

- **Proposed** — authored, not yet reviewed.
- **Accepted** — approved; implementation may proceed.
- **Superseded by NNN** — reversed or replaced; keep the original file, add a pointer to the replacement.
- **Archived** — no longer applies; prefix title with [Archived], add a one-line note under Status.

When a PR implements an Accepted DR, flip its status in the same PR. A DR should never linger in Proposed once the corresponding code ships.

### Open ADRs block design — STOP and decide first

A **Proposed** ADR is a decision that has *not* been made. It is fine for an ADR to sit in Proposed
indefinitely — until its subject matter actually comes up.

**Before designing any feature or sub-issue, check `docs/decisions/` for a Proposed ADR whose scope
touches the design. If one exists, STOP.** Do not design around it, do not pick a plausible default,
do not leave the choice implicit for later. **Work through the decision with the operator and get the
ADR Accepted first**, then continue the design on top of the accepted decision.

Concretely, when you start a design:

1. Scan `docs/decisions/` and list every ADR still marked **Proposed**.
2. For each, ask: *would this design have to assume an answer this ADR is meant to decide?* If yes,
   the ADR is in scope.
3. If any in-scope ADR is still Proposed, **halt the design**, surface it to the operator, and drive
   it to Accepted (write the Decision, flip the status) **before** writing any design that depends on
   it. An open-ended design that silently assumes an undecided ADR is the exact failure this rule
   exists to prevent.

## Feature & Bug Workflow

Every feature or bug fix starts as a **GitHub Issue**. The issue URL is the task's primary reference key — put it in the branch name, the commit trailer, and the PR body. Designs live in the issue body, not in the repo tree.

**This supersedes any skill or tool that tells you to write a design/spec file** — the design goes in the issue body, and you iterate it *in place* there rather than in a repo file (see [Commands](#commands)).

**Reuse existing issues; don't proliferate them.** Before `gh issue create`, check whether an issue already covers the work. If a milestone or feature issue already exists, attach sub-issues **to it** (`--parent <that issue>`) rather than minting a new intermediate parent. **Do not create a new top-level or parent issue without human approval** — creating issues is outward-facing; confirm first.

###  What each part is for

- **Issue body** — the design spec; the single source of truth.
- **Labels** — one `type/{feature,bug,chore}` and one `status/*` lifecycle label.
- **Sub-issues** — the implementation plan, one per step. Create them with `gh issue create --parent <n>` (or link an existing issue with `gh issue edit`); their open/closed state drives the parent's progress bar.
- **Comments** — status updates. Post one when you start a sub-issue and one (with the commit or PR reference) when you finish it.
- **Closed** — done.

###  Lifecycle

1. **`status/design`** — create the parent issue from the Feature or Bug template and write the design into the body. No sub-issues or code yet.
2. **Approval gate** — a human reviews the design in the issue body. If changes are requested, revise the body **in place** (see [Commands](#commands)) and re-request review — never spin up a new issue for a revision. On approval, move `status/design` → `status/ready`.
3. **`status/ready`** — create a sub-issue per plan step and start implementation. Move the parent to `status/in-progress`.
4. **Per sub-issue** — comment that you've started, do the work on a branch, comment the commit/PR reference, then close the sub-issue.
5. **`status/review`** — when every sub-issue is closed, open a PR with `Closes #<parent>` and move the parent to `status/review`.
6. Merging the PR closes the parent. Done.

### Issue templates

The spec structure lives in `.github/ISSUE_TEMPLATE/` (`feature.md`, `bug.md`). Opening an issue from the GitHub **New issue** page seeds the headings and applies the `type/*` + `status/design` labels automatically. From the CLI, fill the same headings and pass the labels explicitly (below).

### Commands

```bash
# Create the parent issue (design stage). The body-file is a scratch input, not a stored
# design — keep it in .workspace/ (gitignored), never in the repo tree.
gh issue create --title "<title>" --label type/feature,status/design --body-file .workspace/design.md

# Update the design in an existing issue — the normal way a design evolves (e.g. after
# review feedback). Pull the current body, edit it, push it back in place; no new issue.
gh issue view <n> --json body --jq '.body' > .workspace/issue-<n>.md   # pull current design
$EDITOR .workspace/issue-<n>.md                                        # revise it
gh issue edit <n> --body-file .workspace/issue-<n>.md                  # push the revision back

# Advance the lifecycle
gh issue edit <n> --remove-label status/design --add-label status/ready

# Create a sub-issue directly under the parent
gh issue create --title "<step>" --label type/feature --parent <parent#>

# ...or link/unlink an existing issue
gh issue edit <parent#> --add-sub-issue <child#>
gh issue edit <parent#> --remove-sub-issue <child#>

# See the plan and its progress
gh issue view <parent#> --json subIssuesSummary --jq '.subIssuesSummary | "\(.completed)/\(.total) done"'
gh issue view <parent#> --json subIssues --jq '.subIssues.nodes[] | "#\(.number) [\(.state)] \(.title)"'

# Post status updates and close
gh issue comment <n> --body "Started."
gh issue comment <n> --body "Done in <commit-sha>."
gh issue close <n>
```

## Release Process

Releases are driven by git tags. Do not create releases manually with `gh release create`.

1. Bump the version in `package.json`.
2. Add a changelog entry to `CHANGELOG.md` under a new version heading.
3. Commit and push to main.
4. Create and push a semver tag: `git tag v<version> && git push origin v<version>`.
5. The `release.yml` workflow handles everything else: builds, security scan, multi-arch Docker image push (with `latest`, semver, and major/minor tags), SBOM generation, and GitHub release creation with artifacts.

The `docker-build-push.yml` workflow runs on every push to main and publishes intermediate Docker images (`main`, `sha-*`, timestamp tags). It prunes old non-release images automatically after each build. The weekly `prune-ghcr.yml` workflow handles any remaining cleanup.
