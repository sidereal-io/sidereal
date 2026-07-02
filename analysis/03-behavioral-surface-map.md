# 3. Behavioral Surface Map

Every externally observable surface of Sidereal. Format per entry: purpose · actor · inputs · outputs · side effects · errors · provenance · confidence.

## 3.1 UI routes (single-page app, dark theme, client-side routing)

| Route | Purpose | Provenance | Conf |
|---|---|---|---|
| `/` | Gallery home: searchable/filterable image grid, sidebar (stats, Astrometry status, recent activity, popular tags), image detail overlay. Deep link `/?image=ID` opens the overlay directly; closing it removes the param. | SRC-33, SRC-36, TEST-01 | High |
| `/targets` | Imaged-target browser (grouped images + catalog enrichment) | SRC-33, SRC-37 | Medium |
| `/equipment` | Equipment catalog CRUD + equipment groups | SRC-33, TEST-06 | High |
| `/plate-solving` | Plate-solving dashboard: 5 stats cards, image selection with search + "only unsolved" filter, bulk submit | SRC-33, TEST-07 | High |
| `/sky-map` | All-sky map (Aladin) with markers for plate-solved images; "View in Gallery" navigates to `/?image=ID` | SRC-33, TEST-08 | High |
| `/locations` | Saved observing-site management | SRC-33 | Medium |
| `/admin` | Admin settings: Immich, Astrometry, sidecar, catalog, database, notifications, app sections; single Save button | SRC-33, TEST-02, TEST-09 | High |
| any other path | 404 page: "404 Page Not Found" card | TEST-04 | High |

All pages share a header (title "Sidereal", logo, nav links, **Sync Immich** button, notification bell, GitHub link to `https://github.com/mstelz/sidereal`, admin gear). Pages render their shell even when every API call fails (TEST-04, TEST-05).

## 3.2 HTTP endpoints (all under `/api`, JSON unless noted; no authentication)

### Images
| Method/Path | Purpose | Notes | Provenance | Conf |
|---|---|---|---|---|
| GET `/api/images` | List images with optional filters `objectType`, `tags` (repeatable, OR), `plateSolved` (`true`/other), `constellation`, `equipmentId` | Returns full image records, no pagination | SRC-03, SRC-21, API-08 | High |
| GET `/api/images/:id` | Single image | 404 `{"message":"Image not found"}` | SRC-03, API-08 | High |
| PATCH `/api/images/:id` | Partial update of any image field (no field whitelist/validation) | `updatedAt` bumped | SRC-03, API-13 | High |
| GET `/api/images/:id/thumbnail` · `/preview` · `/original` | Serve processed image bytes | Immutable 1-year cache, weak ETag `W/"{id}-{size}-{mtime}"`, 304 support, `?download=1` → attachment, Range only on `original` (206/416); 404 + `backfillPending:true` + `X-Sidereal-Backfill: pending` header when file absent | SRC-03, API-12, TEST-03 | High |
| GET `/api/images/:id/plate-solving-job` | Latest successful solve job metadata for image | 400 if not solved | SRC-03 | High |
| GET `/api/images/:id/annotations` | Stored solve annotations + calibration + image dimensions | 400 if not solved / no result | SRC-03 | High |
| GET/POST `/api/images/:id/equipment`, PUT/DELETE `/api/images/:imageId/equipment/:equipmentId` | Link/unlink/annotate equipment on an image | Link changes trigger summary recompute | SRC-03, SRC-21, API-13 | High |
| GET/POST `/api/images/:id/acquisitions`, PUT/DELETE `/api/images/:imageId/acquisitions/:id` | Acquisition session entries per image | `frameCount`+`exposureTime` required; every change recomputes image summary fields | SRC-03, SRC-21, SRC-38, API-13 | High |
| GET `/api/images/:id/sidecar` | Fetch XMP sidecar content (`application/xml`); `?download=true` → attachment `{filename}.xmp` | 404 if none exists | SRC-03, SRC-23 | High |

### Plate solving
| Method/Path | Purpose | Provenance | Conf |
|---|---|---|---|
| POST `/api/plate-solving/images/:id/plate-solve` | Synchronous full solve workflow (submit→poll→apply); 400 when solving disabled/unconfigured | SRC-06, API-10 | High |
| GET `/api/plate-solving/jobs` | All solve jobs | SRC-06 | High |
| POST `/api/plate-solving/bulk` | Submit many images (fire-and-forget per image); per-image success/error results; skips already-solved | SRC-06, API-10 | High |
| POST `/api/plate-solving/update/:jobId` | Re-check one job against Astrometry.net and apply results; broadcasts WS update | SRC-06, SRC-18 | High |

### Equipment & groups
| Method/Path | Purpose | Provenance | Conf |
|---|---|---|---|
| GET/POST `/api/equipment`, PUT/DELETE `/api/equipment/:id` | Equipment CRUD; `name`+`type` required (400) | SRC-08, API-13 | High |
| GET/POST `/api/equipment-groups`, GET/PUT/DELETE `/api/equipment-groups/:id` | Group CRUD, responses include `members` | SRC-09 | High |
| PUT `/api/equipment-groups/:id/members` | Replace membership (array required → 400) | SRC-09 | High |
| POST `/api/equipment-groups/:id/apply/:imageId` | Assign all group members to image (idempotent; only missing links added) | SRC-09, SRC-21 | High |

### Catalog & targets
| Method/Path | Purpose | Provenance | Conf |
|---|---|---|---|
| GET `/api/catalog/browse` | Paged/filterable catalog browse; `{items,total,page,pageSize}`; limit clamped 1–100 | SRC-07, SRC-20, API-09 | High |
| GET `/api/catalog/search?q=` | Autocomplete (≤20; matches name/messier/common names/identifiers, raw + normalized query) | SRC-07, SRC-20, API-09 | High |
| GET `/api/catalog/status` | `{count,lastUpdated,commitSha}` | API-06 | High |
| GET `/api/catalog/thumbnail/:name` | DSS survey JPEG for object; on-disk cache, then served statically; concurrency-limited (25) upstream fetches | SRC-07 | High |
| GET `/api/catalog/:name` | Single catalog object by exact name | API-09 | High |
| POST `/api/catalog/load` | Full re-download + replace of OpenNGC catalog from GitHub | SRC-07, SRC-20, RUN-04 | High |
| POST `/api/catalog/check-updates` | Compare stored commit SHA vs GitHub latest | SRC-07, SRC-20 | High |
| POST `/api/catalog/backfill-targets` | Match untargeted images' tags to catalog; sets `targetName` | SRC-07 | High |
| GET `/api/targets`, GET `/api/targets/:name` | Target summaries (grouped by `targetName`||`title`, catalog-enriched, sorted by image count then recency) and single detail | SRC-11, SRC-38, API-13 | High |
| GET/PUT/DELETE `/api/user-targets[/:catalogName]` | Personal notes/tags per catalog object (upsert semantics) | SRC-12, API-13 | High |

### Sources & ingestion
| Method/Path | Purpose | Provenance | Conf |
|---|---|---|---|
| GET `/api/sources` | List registered sources (`local`, `url`, `immich`) with `{displayName, sourceType, lastSync:null, imageCount, healthy}` | SRC-14, SRC-27, API-07 | High |
| POST `/api/sources/upload` (multipart `file`) | Ingest local file; 201 `{imageId,filename,sourceType:"local",sourceId}`; 413 over 500 MB; content-hash dedup (re-upload → same id, still 201) | SRC-14, SRC-29, API-11 | High |
| POST `/api/sources/ingest-url` `{url}` | Ingest from http(s) URL; 201; URL-hash dedup | SRC-14, SRC-30, API-10 | High |
| GET `/api/sources/:type/status`, POST `/api/sources/:type/test` | Per-source status/connection test; 404 unknown type | SRC-14, API-10 | High |

### Immich integration
| Method/Path | Purpose | Provenance | Conf |
|---|---|---|---|
| POST `/api/immich/sync-immich` | Full synchronous sync; `{syncedCount,removedCount,message}`; 500 with generic message when unconfigured | SRC-05, SRC-28, API-10 | High |
| POST `/api/immich/test-immich-connection` | Test host+key (accepts redacted key placeholder → uses stored); 400 missing/invalid, non-JSON upstream → 500 explanatory | SRC-05, API-10 | High |
| POST `/api/immich/albums` | List Immich albums `{id, albumName}` for album-scoped sync selection | SRC-05 | High |
| POST `/api/immich/backfill` | Kick off original-file backfill; 200 "Backfill started" (fire-and-forget), 409 "Backfill already running" | SRC-05, SRC-32, API-10 | High |
| GET `/api/assets/:assetId/:type` | Proxy Immich asset bytes (`thumbnail`\|`original`\|`fullsize`; query forwarded); 503 when Immich unconfigured | SRC-15 | High |

### System & admin
| Method/Path | Purpose | Provenance | Conf |
|---|---|---|---|
| GET/POST `/api/admin/settings` | Read (API keys masked `••••`+last4) / write full settings document; redacted keys sent back are swapped for stored values; Immich host must be http(s) URL | SRC-04, API-03, API-10 | High |
| POST `/api/test-astrometry-connection` | Validate Astrometry.net key via login | SRC-04, API-10 | High |
| GET `/api/stats` | `{totalImages, plateSolved, totalHours, uniqueTargets, totalEquipment, objectTypeCounts, plateSolvingStats{total,pending,successful,failed}}` | API-02 | High |
| GET `/api/tags` | Top-20 relevant tags with counts | SRC-04, SRC-26 | High |
| GET `/api/constellations` | Sorted distinct constellations across images | SRC-04 | High |
| GET `/api/notifications` | Unacknowledged notifications | SRC-04, API — empty list observed | High |
| POST `/api/notifications/:id/acknowledge`, POST `/api/notifications/acknowledge-all` | Acknowledge; broadcasts `notifications-updated` | SRC-04 | High |
| GET `/api/admin/cron-jobs` | **Currently returns 500 at runtime** (job objects unserializable) — see Conflict Log | API-04 | High |
| GET `/api/admin/database` | `{type, path?, sizeBytes?, lastModified?}` | API-05 | High |
| GET `/api/admin/database/backup` | Stream SQLite file as attachment `sidereal-backup-{timestamp}.db`; 400 on PostgreSQL | SRC-04 | High |
| GET `/api/health` | `{status, timestamp, uptime, database, worker{enabled,running,pid,restartAttempts}, version, nodeVersion}`; 503 when DB check fails | API-01 | High |
| GET `/api/*` (unmatched) | 404 `{"message":"Not found"}` | API-08 | High |
| GET non-API path | SPA `index.html` (prod build) or plain-text "Frontend not built…" 404 | SRC-01 | High |

## 3.3 WebSocket / event stream

- Endpoint: `ws(s)://<host>/ws`. Server pushes JSON text frames `{"event": string, "data": object}`. No client→server protocol is consumed by the server (the worker process *sends* frames over the same socket; the server does not route them — see Open Questions Q4). Provenance: SRC-25, SRC-31, WS-01. Confidence: High (connection), Medium (worker-relay effectiveness).
- Events broadcast to all clients:
  | Event | Payload | Emitted when | Provenance |
  |---|---|---|---|
  | `plate-solving-update` | `{jobId, status, imageId?, result?, message?}` | Job submitted (`processing`), succeeded (`success` + calibration+annotations), failed (`failed` + error details), manual status re-check | SRC-18, SRC-06 |
  | `source-sync-complete` | `{sourceType:"immich", success, message, syncedCount?, removedCount?}` | Scheduled Immich sync finishes (success or failure) | SRC-19 |
  | `notifications-updated` | `{}` | Any acknowledge action | SRC-04 |
  | `backfill-progress` | `{total, processed, failed, skipped?}` | Backfill start and after each batch | SRC-32 |
  | `backfill-complete` | `{processed, failed, skipped, message}` | Backfill finishes | SRC-32 |
- Client behavior: singleton connection shared across views, reconnects with exponential backoff 1s→30s, refetches gallery/stats on `plate-solving-update` and `source-sync-complete` (SRC-34, SRC-36).

## 3.4 Configuration surfaces

1. **Admin settings document** (persisted; authoritative at runtime) — full shape and defaults in §5 SPEC-CFG-1. Read/write via `/api/admin/settings`; catalog metadata (`catalog_lastUpdated`, `catalog_commitSha`, `catalog_objectCount`) stored in the same key-value store (SRC-20).
2. **Environment variables** (process-level): `PORT` (default 5000), `NODE_ENV`, `DATABASE_URL` (presence ⇒ PostgreSQL), `SQLITE_DB_PATH` (default `local.db` dev / `/app/config/sidereal.db` prod), `STORAGE_PATH` (default `./data/images` dev / `/app/data/images` prod), `THUMBNAIL_CACHE_DIR` (default `./data/thumbnails` dev / `/app/cache/thumbnails` prod), `XMP_SIDECAR_PATH` (overrides sidecar output path), `SKIP_DB_INIT=1` (test-only). Worker standalone: `ASTROMETRY_API_KEY`/`ASTROMETRY_KEY`, `ASTROMETRY_CHECK_INTERVAL`, `ASTROMETRY_POLL_INTERVAL`, `PLATE_SOLVE_MAX_CONCURRENT`, `ASTROMETRY_AUTO_RESUBMIT`. Container: `PUID`, `PGID`, `ENABLE_PLATE_SOLVING`, `AUTO_DB_MIGRATE_*`, `SIDEREAL_PORT` (compose port map). Provenance: SRC-01/02/22/23/31, SRC-07, DOCKER-02/03, DOC-04. Confidence: High.
3. **Precedence**: admin settings (DB) > env defaults for feature config; `XMP_SIDECAR_PATH` env overrides the stored sidecar output path (SRC-17). Worker standalone mode activates only when the DB has no Astrometry key but env has one (SRC-31).

## 3.5 Background jobs

| Job | Schedule/trigger | Behavior | Provenance | Conf |
|---|---|---|---|---|
| Immich sync | Cron from `immich.syncFrequency` (default `0 */4 * * *`) | Runs full sync; WS `source-sync-complete`; on failure creates error notification "Immich Sync Failed" | SRC-19, RUN-03 | High |
| Notification cleanup | `0 2 * * *` | Deletes acknowledged notifications older than 30 days | SRC-19, SRC-21, RUN-03 | High |
| Orphan image sweep | `0 3 * * *` | Removes processed-image directories whose DB record no longer exists | SRC-19, SRC-22, RUN-03 | High |
| Plate-solve worker loop | Continuous, every `checkInterval` s (default 30) | Auto-submits unsolved images (respecting `maxConcurrent`, default 3, and `autoResubmit` for failed ones), then polls `processing` jobs and applies results | SRC-31 | High (source) |
| Image backfill | On startup + `POST /api/immich/backfill` | Re-downloads originals for Immich images missing local storage; 4-way concurrent; single-flight; WS progress | SRC-32, RUN-03, API-10 | High |
| Auto catalog load | On startup when catalog empty | Downloads and loads OpenNGC (~14k objects) | SRC-01, RUN-04 | High |

## 3.6 External-service integrations

| Service | Direction | Purpose | Provenance |
|---|---|---|---|
| Immich (`{host}/api/...`, header `X-API-Key`) | outbound | Album list (`GET /api/albums`), album assets (`GET /api/albums/{id}`), metadata search paging (`POST /api/search/metadata` with `{size:1000,page,type:'IMAGE'}`), original download (`GET /api/assets/{id}/original`), asset proxy passthrough | SRC-28, SRC-15, SRC-32 |
| Astrometry.net (`https://nova.astrometry.net/api/...`) | outbound | login (api key → session), upload (multipart of local *preview* JPEG), submission status, job status, calibration, annotations, machine tags | SRC-18 |
| OpenNGC on GitHub (raw.githubusercontent.com + api.github.com) | outbound | `NGC.csv` + `addendum.csv` download; latest commit SHA for update checks | SRC-20, RUN-04 |
| CDS hips2fits (`alasky.cds.unistra.fr`) | outbound | DSS2 color survey thumbnails (300×200 JPEG, FOV from object major axis ×1.5, min 0.05°, default 0.25°) | SRC-07 |

## 3.7 File/storage behavior

- **Processed images**: `{STORAGE_PATH}/processed/{id % 1000, zero-padded 3}/{id}/` containing `{id}_original.{ext}` (ext sanitized to ≤10 alnum chars, default `jpg`), `{id}_preview.jpg` (≤1920px longest side, JPEG q85), `{id}_thumb.jpg` (≤250px, q80). EXIF orientation baked in. Writes are atomic (temp dir + rename); failures roll back both files and DB record. 500 MB max original. (SRC-22, FS-01, API-11)
- **XMP sidecars**: `{outputPath}/[yyyy-mm/]{filename}.xmp` — date subdir when `organizeByDate` and image has a capture date; read side checks date path then flat path. (SRC-23)
- **DSS thumbnail cache**: `{THUMBNAIL_CACHE_DIR}/{sanitized-name}.jpg`; cache hits served as static files. (SRC-07)
- **SQLite DB file**: at `SQLITE_DB_PATH`; downloadable via backup endpoint. (SRC-02, API-05)

## 3.8 Database-visible behavior

Persisted entities (behavioral view; see §9): image, equipment, image↔equipment link, acquisition entry, plate-solving job, admin setting (key/JSON-value), notification, equipment group + membership, catalog object, location, user target. Deleting an image also deletes its solve jobs, equipment links, acquisitions, and local files (SRC-21). No API endpoint deletes an image — removal happens only via Immich sync reconciliation (SRC-28; see Open Questions Q2).

## 3.9 Docker/runtime behavior

- Single container, port 5000, healthcheck `curl -f http://localhost:5000/api/health` (30s interval, 40s start period).
- Entrypoint: remaps user to `PUID`/`PGID` (default 1001), chowns `/app/{config,logs,sidecars,cache,dist,data}`, waits up to 60s for PostgreSQL when `DATABASE_URL` set, runs PG migrations (non-fatal on failure), optional one-time cross-DB migration via `AUTO_DB_MIGRATE_FROM/TO` (marker file `/app/config/.auto-db-migrated`), then supervises **two processes**: main server and (unless `ENABLE_PLATE_SOLVING=false`) the worker; dead worker is restarted every 30s check; dead main process kills the container. SIGTERM/SIGINT → graceful stop of both.
- Volumes: config (DB), logs, sidecars, cache, images, plus external Immich upload volume mount. (DOCKER-01..04)

## 3.10 Operational commands

| Command | Behavior | Provenance |
|---|---|---|
| `npm run dev` | server (watch) + Vite frontend + worker concurrently | DOC-02, package.json |
| `npm run dev:server` / `dev:worker` / `dev:worker:standalone` | individual processes; standalone uses `.env.worker` | package.json |
| `npm run build` / `start` / `start:worker` | production build & run (`dist/index.js`, `dist/worker.js`) | package.json, DOCKER-02 |
| `npm run check` / `test` / `test:server` / `test:e2e` | quality gates (RUN-01/02 pass) | package.json |
| `npm run db:migrate`, `seed`, `seed:reset` | manual migration/seed helpers | package.json |
