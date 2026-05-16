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

## Environment Variables

| Variable | Default | Description |
|---|---|---|
| `PORT` | `5000` | Server port |
| `NODE_ENV` | `development` | Environment |
| `DATABASE_URL` | *(unset)* | PostgreSQL connection string; omit to use SQLite |

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

## Workflow

This project follows the Jesse Vincent **Superpowers** development methodology, with four sidereal-specific overrides. AI agents MUST use both:

1. **`superpowers:brainstorming`, `writing-plans`, `executing-plans` (or `subagent-driven-development`), and `finishing-a-development-branch`** — the standard methodology
2. **`sidereal-workflow-overrides`** — the project's overlay (spec on GitHub issue, plan in `.workspace/plans/`, ADR before PR, `Closes #<n>` in PR body)

When the user asks "what's next?" or starts a workflow without naming an issue, use **`sidereal-pick-next-issue`** first.

### Hard rules

- **No vibe coding.** Never edit code without an `spec/ready` issue and a plan file in `.workspace/plans/`.
- **Mandatory TDD.** Every feature or bug fix follows Red-Green-Refactor. Code without a test baseline will be discarded.
- **Never set `spec/ready` yourself.** Only a human applies that label.

See the `sidereal-workflow-overrides` skill for the full override details.

## Decision Records

When you encounter a design decision with multiple valid approaches, create a decision record before implementing. Decision records ensure developers can review trade-offs and make informed choices rather than discovering baked-in assumptions after the fact.

### When to create a decision record

- Trade-offs between simplicity and flexibility (e.g., hardcoded defaults vs. config)
- Anything where a reasonable person could argue for a different approach

### Decision record format

New ADRs follow `docs/decisions/ADR-000-template.md`. Number sequentially (`ADR-NNN-short-slug.md`) and keep the slug terse.

### Rules
- Do NOT implement a decision before it's recorded. Write the record, set status to Proposed, and let a developer accept it.
- Number sequentially: 001, 002, etc.
- Keep options concrete — include code snippets, interface sketches, or config examples where they clarify trade-offs.
- If a decision is later reversed, set status to Superseded by NNN and create a new record explaining why.

### Lifecycle

Every DR moves through these states:

- **Proposed** — authored but not reviewed. Do not implement.
- **Accepted** — reviewed and approved. Implementation may proceed (or has).
- **Superseded** by NNN — the decision has been reversed or replaced. Keep the original file; add a pointer line at the top to the replacement.
- **Archived** — the decision no longer applies (feature removed, approach abandoned without replacement). Leave the file in place; prefix the title with [Archived] and add a one-line note under Status.

When you open a PR that implements an Accepted DR, flip its status in the same PR. When you supersede a DR, do it in the PR that introduces the replacement. A DR should never linger in Proposed once the corresponding code ships — treat a mismatched status as a correctness bug on par with stale docs.

## Release Process

Releases are driven by git tags. Do not create releases manually with `gh release create`.

1. Bump the version in `package.json`.
2. Add a changelog entry to `CHANGELOG.md` under a new version heading.
3. Commit and push to main.
4. Create and push a semver tag: `git tag v<version> && git push origin v<version>`.
5. The `release.yml` workflow handles everything else: builds, security scan, multi-arch Docker image push (with `latest`, semver, and major/minor tags), SBOM generation, and GitHub release creation with artifacts.

The `docker-build-push.yml` workflow runs on every push to main and publishes intermediate Docker images (`main`, `sha-*`, timestamp tags). It prunes old non-release images automatically after each build. The weekly `prune-ghcr.yml` workflow handles any remaining cleanup.


