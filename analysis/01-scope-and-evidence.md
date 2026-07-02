# 1. Scope

## Repository version

- **Repository**: `sidereal` monorepo (working directory root)
- **Commit analyzed**: `75e675d91e81187911311455072f5e6bed858eca` (branch `main`, merge of PR #194 "Immich legacy removal")
- **Application version**: `0.10.0` (`package.json`)
- **Working-tree deviation**: `apps/server/src/db.ts` has an uncommitted local edit that comments out a try/catch around SQLite initialization. Behavioral difference: at HEAD, any SQLite init failure is masked as a generic "No database configuration found" error; in the analyzed working tree the underlying error propagates. All other analysis matches HEAD. (`package-lock.json` also modified; not behaviorally relevant.)

## Runtime environment used for observations

- Linux 6.17, Node.js v24.10.0, npm workspace install already present.
- Server exercised twice on port **5799** (instead of default 5000) via `tsx apps/server/src/index.ts`:
  1. Against the repo's pre-existing `local.db` (RUN-03).
  2. Against a **fresh, isolated SQLite database and storage root** in a scratch directory using `SQLITE_DB_PATH`, `STORAGE_PATH`, `XMP_SIDECAR_PATH` overrides (RUN-04) — this validated first-run migration, catalog auto-load, upload, image processing, serving, and metadata workflows without touching developer data.
- Live outbound network was available: the fresh-start catalog auto-load fetched the OpenNGC catalog from GitHub successfully.

## Commands run

| Command | Result |
|---|---|
| `npm run check` (TypeScript) | Pass, no output (RUN-01) |
| `npm run test` (unit) | 3/3 pass — `imageUrl` helper tests (RUN-02) |
| `tsx apps/server/src/index.ts` (existing DB) | Clean startup; see RUN-03 log excerpts |
| `tsx apps/server/src/index.ts` (fresh DB) | Migrations applied; catalog auto-loaded 14,033 objects (RUN-04) |
| ~40 live HTTP probes with `curl` | API-01 … API-13 |
| WebSocket client connect to `/ws` | Connected (WS-01) |

## Evidence inspected

All of: `apps/server/src/**` (routes, services, workers, entry, DB init), `apps/client/src/` (routing, pages, hooks, query client, header — component internals sampled), `packages/shared/src/` (both DB schemas, types, image/catalog/visibility utilities), `tools/migrations/sqlite/`, `tests/e2e/*.spec.ts`, `docker/` (Dockerfile, startup.sh, compose files), `.env.example`, `.env.worker.example`, `package.json`, `docs/api.md`, `docs/MIGRATION.md`, `CLAUDE.md`.

## Explicitly out of scope

- **Not observed live**: Immich server interaction (no Immich instance available), Astrometry.net solve lifecycle end-to-end (no API key; submission/polling behavior is source-inferred), PostgreSQL mode, Docker container runtime (artifacts inspected, container not built/run), Playwright E2E execution (specs read as behavioral assertions, not executed), the standalone worker process loop, cron job firings (schedules observed, executions not awaited).
- **Not part of the product surface**: internal module layout, class/service names, Drizzle/Hono/React specifics, build tooling, CI/CD pipelines, the beads issue tracker, unfinished feature docs under `docs/features/` (sessions, calibration files, NINA plugin, dashboard — no corresponding runtime surface found).
- **Auth**: the application has **no authentication or authorization of any kind**; there is nothing to specify beyond that fact (see Error Contract §10).

---

# 2. Evidence Inventory

Confidence: H = high, M = medium. Hash for all `SRC/SCHEMA/MIG/TEST/DOC/DOCKER` items is the repo commit `75e675d`.

## Source inspection (SRC)

| ID | Artifact | Path | Relevance | Conf |
|---|---|---|---|---|
| SRC-01 | Server entry/startup | `apps/server/src/index.ts` | Startup order, static serving, SPA fallback, shutdown, WS wiring | H |
| SRC-02 | DB initialization | `apps/server/src/db.ts` | SQLite/PG selection, migration-on-start, `SQLITE_DB_PATH` | H |
| SRC-03 | Image routes | `apps/server/src/routes/images.ts` | Image CRUD-ish API, byte serving, acquisitions, equipment links, sidecar fetch | H |
| SRC-04 | System routes | `apps/server/src/routes/system.ts` | Admin settings, connection tests, stats, tags, constellations, notifications, cron status, DB info/backup, health | H |
| SRC-05 | Immich routes | `apps/server/src/routes/immich.ts` | Manual sync, connection test, album list, backfill trigger | H |
| SRC-06 | Plate-solving routes | `apps/server/src/routes/plate-solving.ts` | Solve submit, jobs list, bulk submit, job status update | H |
| SRC-07 | Catalog routes | `apps/server/src/routes/catalog.ts` | Browse/search/status/load/check-updates/backfill-targets, DSS thumbnail proxy+cache | H |
| SRC-08 | Equipment routes | `apps/server/src/routes/equipment.ts` | Equipment CRUD | H |
| SRC-09 | Equipment-group routes | `apps/server/src/routes/equipment-groups.ts` | Group CRUD, membership, apply-to-image | H |
| SRC-10 | Location routes | `apps/server/src/routes/locations.ts` | Location CRUD | H |
| SRC-11 | Target routes | `apps/server/src/routes/targets.ts` | Target summaries and detail | H |
| SRC-12 | User-target routes | `apps/server/src/routes/user-targets.ts` | Personal catalog annotations | H |
| SRC-13 | Sky-map routes | `apps/server/src/routes/sky-map.ts` | Plate-solved marker feed | H |
| SRC-14 | Source routes | `apps/server/src/routes/sources.ts` | Source list/status/test, file upload, URL ingest | H |
| SRC-15 | Asset proxy route | `apps/server/src/routes/assets.ts` | Immich asset proxying | H |
| SRC-16 | Route error helper | `apps/server/src/routes/route-utils.ts` | Error body shape, upstream status forwarding | H |
| SRC-17 | Config service | `apps/server/src/services/config.ts` | Setting keys, defaults, precedence, caching | H |
| SRC-18 | Astrometry service | `apps/server/src/services/astrometry.ts` | Solve submit/poll/result/apply lifecycle, WS events | H |
| SRC-19 | Cron manager | `apps/server/src/services/cron-manager.ts` | Scheduled jobs, failure notifications, WS events | H |
| SRC-20 | Catalog service | `apps/server/src/services/catalog.ts` | OpenNGC load/replace, search, browse, tag→target matching | H |
| SRC-21 | Storage service | `apps/server/src/services/storage.ts` | Persistence semantics: filters, delete cascade, summary recompute, settings upsert, stats | H |
| SRC-22 | Image storage | `apps/server/src/services/image-storage.ts` | Processed-image layout, sizes, limits, atomic write, orphan delete | H |
| SRC-23 | XMP sidecar service | `apps/server/src/services/xmp-sidecar.ts` | Sidecar XML content, path layout, resolution | H |
| SRC-24 | Worker manager | `apps/server/src/services/worker-manager.ts` | Production in-process worker spawn/restart policy | H |
| SRC-25 | WS manager | `apps/server/src/services/ws-manager.ts` | WS endpoint path, broadcast envelope | H |
| SRC-26 | Tag filter rules | `apps/server/src/services/tags-utils.ts` | Which tags survive plate-solve machine-tag ingestion | H |
| SRC-27 | Source registry | `apps/server/src/services/source-registry.ts` | Registered source types: local, url, immich | H |
| SRC-28 | Immich source plugin | `apps/server/src/services/sources/immich.ts` | Sync algorithm, asset mapping, removal, size limits | H |
| SRC-29 | Local upload source | `apps/server/src/services/sources/local-upload.ts` | Content-hash dedup, defaults, rollback on write failure | H |
| SRC-30 | URL upload source | `apps/server/src/services/sources/url-upload.ts` | URL-hash dedup, extension inference | H |
| SRC-31 | Plate-solve worker | `apps/server/src/workers/worker.ts` | Auto-submit loop, concurrency, standalone env mode, WS relay | H |
| SRC-32 | Backfill worker | `apps/server/src/workers/backfill-images.ts` | Immich original backfill, progress events, single-flight | H |
| SRC-33 | Client routing | `apps/client/src/App.tsx` | UI routes | H |
| SRC-34 | Client WS hook | `apps/client/src/hooks/use-socket.ts` | Client reconnect/backoff, event subscriptions | H |
| SRC-35 | Client query layer | `apps/client/src/lib/queryClient.ts` | Filter→query-param mapping, no-retry/no-refetch defaults | H |
| SRC-36 | Home page | `apps/client/src/pages/home.tsx` | Gallery workflow, deep-link `?image=ID`, client-side filters, load-more paging | H |
| SRC-37 | Client API usage sweep | header + admin/equipment/plate-solving/sky-map/targets/locations pages (grep) | Which endpoints each page calls | M |
| SRC-38 | Image summary/target grouping | `packages/shared/src/image-utils.ts` | Summary computation, target grouping/enrichment/sort rules | H |
| SRC-39 | Catalog utilities | `packages/shared/src/catalog-utils.ts` | Name normalization, RA/Dec conversion, CSV parsing, match priority | H |
| SRC-40 | Visibility math | `packages/shared/src/visibility.ts` | Circumpolar/best-month computation (client-side) | H |
| SRC-41 | Image URL helper | `packages/shared/src/utils/image-urls.ts` | Canonical image URL scheme | H |

## Schemas and types (SCHEMA)

| ID | Artifact | Path | Relevance | Conf |
|---|---|---|---|---|
| SCHEMA-01 | SQLite schema | `packages/shared/src/db/sqlite-schema.ts` | All persisted entities and columns (SQLite dialect) | H |
| SCHEMA-02 | PostgreSQL schema | `packages/shared/src/db/pg-schema.ts` | PG parity schema (header mandates sync with SQLite) | H |
| SCHEMA-03 | Legacy schema module | `packages/shared/src/schemas/schema.ts` | Older PG-style definitions still present (pre-`sourceType` era); superseded | M |
| SCHEMA-04 | Shared types | `packages/shared/src/types/index.ts` | Notification shape, equipment types and spec-field catalog | H |
| SCHEMA-05 | Image-source contract | `packages/shared/src/types/image-source.ts` | Source plugin surface: status/connection/ingest result shapes | H |

## Migrations (MIG)

| ID | Artifact | Path | Relevance | Conf |
|---|---|---|---|---|
| MIG-01 | SQLite migration series | `tools/migrations/sqlite/0000…0011 + meta/_journal.json` | 12 sequential migrations incl. `0004_add_original_path`, `0009_local_image_storage`, `0010_add_source_columns`, `0011` (drops `immich_id`) | H |

## Tests (TEST)

| ID | Artifact | Path | Relevance | Conf |
|---|---|---|---|---|
| TEST-01 | Home E2E | `tests/e2e/home.spec.ts` | Gallery, sidebar, overlay, deep-linking assertions | H |
| TEST-02 | Admin E2E | `tests/e2e/admin.spec.ts` | Admin form fields, conditional visibility, save button | H |
| TEST-03 | Image-serving E2E | `tests/e2e/image-serving.spec.ts` | `img src` must be `/api/images/{id}/(thumbnail|preview)`, never `/api/assets/`, cache header asserted | H |
| TEST-04 | Error-handling E2E | `tests/e2e/error-handling.spec.ts` | 404 page; pages render shell with API fully blocked | H |
| TEST-05 | Navigation E2E | `tests/e2e/navigation.spec.ts` | Header consistency, active-link styling, GitHub link URL | H |
| TEST-06 | Equipment E2E | `tests/e2e/equipment.spec.ts` | Form validation states, type dropdown options | H |
| TEST-07 | Plate-solving E2E | `tests/e2e/plate-solving.spec.ts` | Five stats cards, selection UI, unsolved filter | H |
| TEST-08 | Sky-map E2E | `tests/e2e/sky-map.spec.ts` | Marker count subtitle, empty state, view-in-gallery navigation | H |
| TEST-09 | Notifications E2E | `tests/e2e/admin-notifications.spec.ts` | Show-first-5, expand/collapse, acknowledge rules | H |
| TEST-10 | Unit tests (shared) | `packages/shared/src/**/*.test.ts` (run) | imageUrl route scheme | H |
| TEST-11 | Server unit tests | `apps/server/src/**/*.test.ts` (present, not run) | serving, image-storage, source plugins | M |

## Documentation (DOC)

| ID | Artifact | Path | Relevance | Conf |
|---|---|---|---|---|
| DOC-01 | API docs | `docs/api.md` | Public API reference — **partially stale**, see Conflict Log | M |
| DOC-02 | Agent/project guide | `CLAUDE.md` | Commands, env-var table, architecture claims | M |
| DOC-03 | Rename migration guide | `docs/MIGRATION.md` | Volume/DB/user rename compatibility (Skymmich→Sidereal) | H |
| DOC-04 | Env examples | `.env.example`, `.env.worker.example` | Documented env surface, worker standalone mode | H |

## Docker (DOCKER)

| ID | Artifact | Path | Relevance | Conf |
|---|---|---|---|---|
| DOCKER-01 | Image build | `Dockerfile` | Runtime dirs, non-root user, healthcheck, port 5000 | H |
| DOCKER-02 | Container entrypoint | `docker/startup.sh` | PUID/PGID remap, DB wait, PG migrations, auto-migrate, dual-process supervision | H |
| DOCKER-03 | Default compose | `docker-compose.yml` | Volumes, env, healthcheck, external Immich upload volume | H |
| DOCKER-04 | PG override compose | `docker-compose.postgres.yml` | PG service, DATABASE_URL wiring | H |

## Runtime observations (RUN / API / WS / FS)

| ID | Source | Relevance | Conf |
|---|---|---|---|
| RUN-01 | `npm run check` — pass | Type gate green at analyzed commit | H |
| RUN-02 | `npm run test` — 3/3 pass | Unit suite content and result | H |
| RUN-03 | Server startup log (existing DB, port 5799) | Startup sequence, cron schedules (`0 */4 * * *`, `0 2 * * *`, `0 3 * * *`), "Nothing to backfill", catalog-already-loaded path | H |
| RUN-04 | Server startup log (fresh DB) | Migrations run automatically; catalog auto-loads 14,033 objects from network | H |
| API-01 | `GET /api/health` | Full health body incl. worker sub-status | H |
| API-02 | `GET /api/stats` | Exact stats body field names | H |
| API-03 | `GET /api/admin/settings` | Full default config document | H |
| API-04 | `GET /api/admin/cron-jobs` → **500** | Endpoint fails at runtime (serialization) | H |
| API-05 | `GET /api/admin/database` | DB info body | H |
| API-06 | `GET /api/catalog/status` | `{count, lastUpdated, commitSha}` | H |
| API-07 | `GET /api/sources` | Three sources with status shape | H |
| API-08 | `GET /api/images`, `/images/999999`, `/images/abc` | Empty list; 404 body; non-numeric id → 404 (not 400) | H |
| API-09 | `GET /api/catalog/browse|search|{name}` | Response shapes incl. `pageSize`, full catalog object | H |
| API-10 | Validation/error probe batch (settings, immich, plate-solving, sources, backfill, astrometry test) | Exact 400/404/409-style bodies and codes | H |
| API-11 | `POST /api/sources/upload` ×2 | 201 ingest result; content-hash dedup returns same id | H |
| API-12 | Image serving probes | 200 with immutable cache + weak ETag; 304 on If-None-Match; 206 ranges on original; 404+`backfillPending` when file missing | H |
| API-13 | Metadata workflow probes | Equipment create/link, acquisitions → summary recompute (`frameCount 15`, `totalIntegration 1`, `filters "Ha, OIII"`, `exposureTime "120s - 300s"`, focal length/aperture from telescope), PATCH targetName, `/targets` grouping+catalog enrichment, user-target upsert, location create | H |
| WS-01 | WebSocket client → `ws://host/ws` | Connection accepted; JSON `{event,data}` envelope (from SRC-25) | H |
| FS-01 | Scratch storage listing after upload | `processed/001/1/1_original.jpg`, `1_preview.jpg`, `1_thumb.jpg` — shard = `id % 1000` zero-padded | H |

**UI-\*** note: no live browser session was run. All UI claims carry `TEST-*` (Playwright assertions, black-box by nature) and `SRC-3x` provenance instead of `UI-*` IDs; treat them as source/test-confirmed rather than screen-observed.
