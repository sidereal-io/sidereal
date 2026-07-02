# 13. Compatibility Requirements

What a replacement implementation must preserve, by consumer.

## 13.1 Existing frontend (if the current SPA is kept against a new backend)

- Every endpoint in §6 with exact JSON field names and status codes; the frontend string-parses errors as `"{status}: {body}"` and renders `message`.
- Image URLs `/api/images/{id}/{thumbnail|preview|original}` (the shared helper hard-codes this scheme; unit-tested).
- WebSocket at `/{ws}` on the same origin/port, `{event, data}` envelope, events: `plate-solving-update`, `source-sync-complete`, `notifications-updated`, `backfill-progress`, `backfill-complete`. The client treats them as refetch triggers — payload shapes may not shrink.
- Masked API-key round-trip protocol (`••••…` sentinel accepted on write and in connection tests).
- The `backfillPending` 404 contract on image bytes.
- SPA fallback for all non-API paths; static assets at `/assets/*`, `/favicon.ico`, `/logo.png`.

## 13.2 Existing API clients / scripts

- Everything in §6, including the quirky-but-public behaviors: 201 on deduplicated ingests, 200-with-`success:false` connection-test outcomes, 409 backfill, 413 upload cap, `pageSize` in browse, `count` in catalog status, stats field names.
- `docs/api.md` is **not** a reliable contract — where it conflicts with this package (Conflict Log C2–C7), observed behavior wins.

## 13.3 Existing persisted data

- SQLite database at migration `0011` (and PG equivalent): all §9.2 tables/columns, uniqueness rules (`admin_settings.key`, `catalog_objects.name`, `user_targets.catalog_name`), epoch-timestamp storage in SQLite, tags as JSON array (SQLite) / text[] (PG), settings as per-section JSON rows including the `catalog_*` metadata keys.
- Image `id` values are load-bearing (URLs, bookmarks/deep links, storage paths) — never renumber.
- `(source_type, source_id)` provenance semantics: Immich asset UUID / content-hash / URL-hash as described in §4.1; sync reconciliation and dedup depend on them.
- Plaintext API keys already exist in deployed databases; a replacement adding encryption must migrate them transparently.

## 13.4 Existing storage layout

- `{STORAGE_PATH}/processed/{zeroPad3(id % 1000)}/{id}/{id}_original.{ext}`, `{id}_preview.jpg`, `{id}_thumb.jpg` — deployed volumes hold terabytes in this shape; readers (serving), writers (ingest/backfill), and the orphan sweep must all agree on it. (The source explicitly marks the processing constants as a cross-implementation contract.)
- Sidecars at `{outputPath}[/yyyy-mm]/{filename}.xmp`; readers must check the dated path first, then flat.
- DSS thumbnail cache filenames: object name sanitized `[^a-zA-Z0-9_-] → _` + `.jpg`.

## 13.5 Docker deployment

- Port 5000; healthcheck `GET /api/health` returning 200.
- Volume mount points: `/app/config` (SQLite DB + auto-migrate marker), `/app/logs`, `/app/sidecars`, `/app/cache`, `/app/data/images`; external Immich upload volume mount tolerated.
- Env contract: `PUID`/`PGID` remapping; `ENABLE_PLATE_SOLVING` gates the worker; `AUTO_DB_MIGRATE_FROM/TO/ONCE/MARKER/RESET_SQLITE` one-time migration protocol; `SIDEREAL_PORT` compose mapping; `DATABASE_URL`/`SQLITE_DB_PATH`/`STORAGE_PATH`/`XMP_SIDECAR_PATH`/`THUMBNAIL_CACHE_DIR` with the defaults in §3.4.
- Two-process model (server + solver worker) with worker auto-restart; a replacement may consolidate processes **only if** health reporting, `ENABLE_PLATE_SOLVING`, and solve throughput semantics are preserved.

## 13.6 Environment variables

All names, defaults, and precedence in §3.4 — including worker-standalone vars (`ASTROMETRY_API_KEY`/`ASTROMETRY_KEY`, `ASTROMETRY_CHECK_INTERVAL`, `ASTROMETRY_POLL_INTERVAL`, `PLATE_SOLVE_MAX_CONCURRENT`, `ASTROMETRY_AUTO_RESUBMIT`) and the standalone-mode activation rule (env used only when DB holds no key).

## 13.7 Immich integration

- Outbound calls: `GET {host}/api/albums[?take=1]`, `GET {host}/api/albums/{id}`, `POST {host}/api/search/metadata` (`{size:1000, page, type:'IMAGE'}` pagination via `nextPage`), `GET {host}/api/assets/{id}/original`, all with `X-API-Key` header — a replacement must remain compatible with the Immich API versions these represent.
- Asset→image field mapping of §4.1 (EXIF names: `fNumber`→`f/…`, `make`+`model`, `lensModel`, `fileCreatedAt`, coordinates, `description`).
- Reconciliation delete semantics (asset gone ⇒ image gone) — users rely on Immich as the source of truth for the `immich` source.

## 13.8 Astrometry.net integration

- nova.astrometry.net API sequence: login (`request-json` form field with `{apikey}`), multipart upload with session, submission polling, job status, `calibration`, `annotations`, `machine_tags` endpoints; a User-Agent header is sent.
- Upload material = preview rendition (≤1920 px JPEG); changing this changes solve behavior and cost.
- Persisted diagnostic URLs (`https://nova.astrometry.net/status/{subId}`, `/annotated_full/{jobId}`) appear in failure results users may have bookmarked.
- Tag-filtering rules (§SPEC-SOLVE-2) shape stored tags; changing them changes `/api/tags` and target matching outcomes on future solves.

## 13.9 User workflows

- All §7 flows, including deep-link `/?image=ID`, the Sync Immich header button, masked-key admin round-trip, notification acknowledge flows, sky-map → gallery navigation, and load-more paging behavior.

## 13.10 Tests

- The existing Playwright suite (`tests/e2e/`) is an executable compatibility contract for the UI (page-object selectors: heading text, placeholder text, button names, `aria-label="Close"`, badge texts "Plate Solved"/"No Plate Data", GitHub URL). A replacement UI that intends to pass it must preserve those user-visible strings and structures.
- Unit contract: `imageUrl(id, size)` route scheme.
- `npm run check`, `npm run test`, `npm run test:e2e` (app on :5173) remain the quality gates.

## 13.11 External catalog

- OpenNGC CSV format (semicolon-delimited, quoted fields, column headers `Name;Type;RA;Dec;Const;MajAx;MinAx;B-Mag;V-Mag;SurfBr;Hubble;M;NGC;IC;Common names;Identifiers`) and the normalization rules (`NGC0001`→`NGC 1`, Messier zero-strip) — stored names in user data (targets, user-targets) were produced by these rules, so matching must remain consistent.
