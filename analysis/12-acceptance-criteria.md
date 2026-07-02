# 12. Acceptance Criteria

Measurable, black-box criteria for a replacement implementation, grouped by surface. Each is verifiable via HTTP, WebSocket, UI, runtime commands, logs, persisted state, file outputs, or container behavior. "Must" = compatibility-breaking if violated; "Should" = user-visible quality parity.

## AC-1 Startup & operations

1. Fresh start with no configuration must yield a serving HTTP endpoint on `PORT` (default 5000) with a passing `/api/health` — no manual DB setup, no manual migration step.
2. Schema upgrades must be applied automatically at startup against an existing SQLite database from any prior 0.x version; startup must fail loudly if migration fails.
3. On first start with network access, the deep-sky catalog must self-populate (>10,000 objects visible via `/api/catalog/status` and `/browse`) without user action; without network the app must still serve.
4. SIGTERM/SIGINT must produce a clean exit after stopping background work.
5. `/api/health` must report overall status, database status, worker status (`enabled/running/pid/restartAttempts`), version, and return 503 when the database is unusable.

## AC-2 HTTP API compatibility

1. Every endpoint, method, path, request shape, response shape, and status code in §6 must behave as tabulated — verified by replaying the §11 vectors marked ✔ and getting byte-compatible JSON field names (e.g. stats uses `plateSolved`/`totalHours`/`uniqueTargets`; browse uses `pageSize`; catalog status uses `count`).
2. All error bodies must be `{"message": string}` with the observed messages for the validation cases in §10.2 (clients may string-match; preserve exact text for the P0 cases).
3. Unmatched `/api/*` must 404 with `{"message":"Not found"}`; unmatched non-API paths must serve the SPA.
4. No endpoint may require authentication (baseline parity; any auth added must be opt-in/off by default).

## AC-3 Images & storage

1. Ingesting a file ≤500 MB via `POST /api/sources/upload` must create exactly one image and three renditions retrievable at `/api/images/{id}/{original|preview|thumbnail}`; preview ≤1920 px, thumbnail ≤250 px, JPEG, EXIF orientation applied.
2. Re-ingesting identical content (local) or an identical URL (url source) must return the existing image id with 201 and create nothing new.
3. Files >500 MB must be rejected (413 on upload; skip-with-log during sync/backfill).
4. Byte serving must provide: 1-year immutable cache headers, ETag + 304 revalidation, byte-range support (206/416) on originals only, `?download=1` attachment mode, and the 404 + `backfillPending:true` + `X-Sidereal-Backfill: pending` contract when the local file is absent.
5. On-disk layout must remain `{STORAGE_PATH}/processed/{zeroPad3(id%1000)}/{id}/{id}_original.{ext}|{id}_preview.jpg|{id}_thumb.jpg` so existing storage volumes remain readable and the orphan-sweep/backfill contracts hold.
6. Failed ingestion must leave no partial image record.
7. Deleting an image (via sync reconciliation) must remove its jobs, equipment links, acquisitions, and rendition directory.

## AC-4 Immich integration

1. With valid config, sync must import all (or selected-album) Immich image assets exactly once each, mapping EXIF to capture fields as in §4.1, and must delete Sidereal images whose assets disappeared upstream; response reports `{syncedCount, removedCount, message}`.
2. Repeat sync with unchanged upstream must be a no-op (0/0).
3. Scheduled sync must run on the configured cron (default every 4 h), broadcast `source-sync-complete`, and persist an error notification on failure.
4. Connection test must accept a masked API key placeholder and validate against the live server with a bounded timeout, distinguishing bad-URL (400), unreachable (500 + cause), and rejected-key (200 `{success:false}`) outcomes.
5. Backfill must be single-flight (409 when running), fire-and-forget over HTTP, report progress via `backfill-progress`/`backfill-complete` WS events, and repair `originalPath` + renditions.
6. `/api/assets/{id}/{type}` must keep proxying Immich assets (validated id/type, 503 when unconfigured, query passthrough).

## AC-5 Plate solving

1. With solving disabled or unkeyed, all solve entry points must refuse with 400 and the configuration message.
2. Submissions must upload the locally stored preview rendition, create a job in `processing`, and broadcast `plate-solving-update`.
3. A successful solve must set all image fields listed in SPEC-SOLVE-2 (including constellation derivation, tag merge with the documented tag-filter rules, and automatic target assignment) and persist the job result (calibration + annotations) retrievable via `/api/images/{id}/annotations`.
4. Failures must persist a `failed` job with a human-readable reason and nova.astrometry.net diagnostic URLs and broadcast the failure.
5. Automatic mode must respect `checkInterval`, `maxConcurrent`, and `autoResubmit` exactly (verify with TV-064), and must pick up settings changes without a process restart.
6. XMP sidecars, when enabled, must be written to `{outputPath}[/yyyy-mm]/{filename}.xmp` with the content contract of SPEC-SOLVE-4 and be retrievable/downloadable via the sidecar endpoint.

## AC-6 Catalog & targets

1. Browse/search must reproduce the query semantics of SPEC-CAT-2 including normalization (`m31` → M 31 → NGC 224), Messier-only filter, magnitude/size bounds, multi-type OR, never-rises filtering by latitude, all four sorts, and the `{items,total,page,pageSize}` envelope with limit clamped to 100.
2. Catalog reload must be a full replace that updates `{count, lastUpdated, commitSha}`; update check must compare against upstream and report `{hasUpdate, currentSha, latestSha}`.
3. `/api/targets` must group by `targetName || title` and enrich from the catalog (name or Messier match) with the exact summary fields and ordering of SPEC-TGT-1.
4. Tag→target matching must be Messier-first, then brightest, using normalized exact-name and Messier cross-reference matching; backfill-targets must only fill images lacking a target and report `{matched, skipped, total}`.
5. User-target annotations must upsert by catalog name, preserve omitted fields on update, and delete idempotently.
6. DSS thumbnails must be served as JPEG with immutable caching, disk-cached after first fetch, 404 for coordinate-less objects, 502 on upstream failure.

## AC-7 Equipment, acquisitions, locations

1. Equipment CRUD must enforce name+type (400) and expose `specifications` as a free-form object round-tripped losslessly.
2. Image–equipment links must carry per-link `settings`/`notes` and merge them into the per-image equipment view.
3. The derived-summary contract must hold exactly (TV-042 numbers): total frames, hours to 3 decimals, distinct filter names joined ", ", uniform vs `min - max` exposure string, telescope focal length/focal ratio flowing to image `focalLength`/`aperture`, recomputed on every acquisition or link change.
4. Group apply must add only missing links and report what was added; membership replace must be exact.
5. Locations must support full CRUD with the §6 shapes.

## AC-8 Notifications, stats, real-time

1. Only unacknowledged notifications are listed; acknowledge (single/all) must broadcast `notifications-updated`; acknowledged items older than 30 days must be purged by a daily job.
2. `/api/stats`, `/api/tags` (top-20, relevance-filtered), `/api/constellations` (sorted distinct) must match §5 SPEC-STAT-1 shapes.
3. A WebSocket endpoint at `/ws` on the same port must broadcast JSON `{event,data}` frames for the five events of §3.3 to **all** connected clients; browser clients must be able to drive UI refresh solely from these events.

## AC-9 Frontend behavior

1. All §7 workflows must function per the E2E-asserted behaviors: header/nav consistency, gallery filters + 12-at-a-time load-more, overlay deep-linking `/?image=ID` (open on load, param removed on close), plate-solve badges, admin form field types and conditional visibility, notification card rules (5-visible, expand, acknowledge-all threshold), sky-map empty state and gallery navigation, equipment form validation gating, 404 page.
2. Gallery images must load exclusively from `/api/images/{id}/(thumbnail|preview)` with long-lived caching; no Immich proxy URLs in the gallery.
3. Every page must render its shell (header + headings) when all API calls fail.
4. API keys must appear masked in the admin UI and saving the form must never erase stored keys.

## AC-10 Persistence & deployment

1. An existing SQLite database file (schema at migration 0011) and an existing `STORAGE_PATH` tree must be readable without data loss; image ids must remain stable.
2. `DATABASE_URL` presence must select PostgreSQL with equivalent behavior; absence must select SQLite at `SQLITE_DB_PATH`/defaults.
3. SQLite backup download must produce a restorable database file; PG must be refused with the documented message.
4. The container must: expose 5000, pass the `/api/health` healthcheck, honor `PUID`/`PGID`, persist config/logs/sidecars/cache/images across restarts via the documented volume mounts, run the solver worker unless `ENABLE_PLATE_SOLVING=false`, restart a dead worker, and die when the main process dies.
5. Environment variables in §3.4 must retain their names, defaults, and precedence (admin settings > env; `XMP_SIDECAR_PATH` overrides stored sidecar path).

## AC-11 Known-defect parity decisions (must be resolved explicitly)

For each item, a replacement must either reproduce the behavior or document the deliberate fix: (a) `/api/admin/cron-jobs` 500 (C1 — fix recommended: return serializable job status); (b) settings-driven sync cron not rescheduled until restart (Q5); (c) success responses for DELETEs of nonexistent resources (C8); (d) non-numeric ids treated as 404 on JSON GETs but 400 on byte routes; (e) `metadataSyncEnabled` settings with no backing endpoints (C2).
