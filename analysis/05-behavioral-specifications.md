# 5. Behavioral Specifications

Each spec: Purpose · Actors · Preconditions · Inputs · Outputs · Validation · Defaults · Side effects · State transitions · Idempotency · Ordering/concurrency · Errors · Edge cases · Security/privacy · Compatibility · Provenance · Confidence. Fields that don't apply are omitted. Error bodies are always `{"message": string}` unless stated (see §10).

---

## SPEC-SYS-1 — Application startup

- **Purpose**: bring the service to a usable state with zero manual steps on a fresh install.
- **Actors**: operator (process start), container supervisor.
- **Inputs**: env vars (`PORT`, `NODE_ENV`, `DATABASE_URL`, `SQLITE_DB_PATH`, `STORAGE_PATH`, …).
- **Behavior (observed order)**: open DB (SQLite file created if absent; pending schema migrations applied automatically before serving) → HTTP server listens on `PORT` (default 5000) → WebSocket endpoint `/ws` available on same port → API routes live → static frontend served from build dir with SPA fallback → scheduled jobs registered (Immich sync per configured cron, notification cleanup 02:00, orphan sweep 03:00 daily) → if the deep-sky catalog is empty, it is downloaded and loaded automatically (network permitting) → a storage backfill check runs (no-op when nothing pending) → in production only, the plate-solving worker is spawned as a child process when solving is enabled+configured.
- **Outputs**: log lines announcing DB choice/path, migration completion, port, environment, DB kind, cron schedules, catalog status, backfill status.
- **Defaults**: port 5000; SQLite at `local.db` (dev) / `/app/config/sidereal.db` (prod).
- **Side effects**: DB file creation, catalog network fetch, possible backfill network fetches.
- **Errors**: catalog auto-load failure and backfill failure are logged but **non-fatal**; migration failure is fatal.
- **Edge cases**: startup with unreachable network still serves the app (catalog stays empty until manual load).
- **Shutdown**: SIGTERM/SIGINT (also uncaught exception/rejection) → stop worker (prod), stop cron, exit 0.
- **Provenance**: SRC-01, SRC-02, RUN-03, RUN-04. **Confidence**: High.

## SPEC-SYS-2 — Static frontend & SPA fallback

- Any non-`/api` GET returns the built `index.html` (client router resolves the view); `/assets/*`, `/favicon.ico`, `/logo.png` served as static files. If the frontend isn't built: plain-text `Frontend not built. Run npm run build first.` with 404. Unmatched `/api/*` GETs return 404 `{"message":"Not found"}`.
- **Provenance**: SRC-01, API-08. **Confidence**: High.

## SPEC-SYS-3 — Health check

- **Purpose**: container/monitoring probe. **Actors**: Docker healthcheck, monitors.
- **Inputs**: `GET /api/health`. **Outputs**: 200 `{status:"healthy", timestamp, uptime, database:"healthy", worker:{enabled,running,pid,restartAttempts}, version, nodeVersion}`; 503 with `status:"unhealthy"` when the DB probe (a stats aggregation) fails.
- **Edge cases**: worker block reflects the in-process production worker manager; in dev it reports `enabled:false, running:false, pid:null`.
- **Provenance**: API-01, SRC-04, DOCKER-01. **Confidence**: High.

## SPEC-CFG-1 — Settings document: read

- **Actors**: admin UI. **Inputs**: `GET /api/admin/settings`.
- **Outputs** (200; observed defaults on a fresh install):
```json
{
  "immich":     { "host": "", "apiKey": "", "autoSync": false, "syncFrequency": "0 */4 * * *",
                  "syncByAlbum": false, "selectedAlbumIds": [], "metadataSyncEnabled": false,
                  "syncDescription": true, "syncCoordinates": true, "syncTags": true },
  "astrometry": { "apiKey": "", "enabled": false, "autoEnabled": false, "checkInterval": 30,
                  "pollInterval": 5, "maxConcurrent": 3, "autoResubmit": false },
  "sidecar":    { "enabled": true, "outputPath": "./sidecars", "organizeByDate": true },
  "app":        { "debugMode": false }
}
```
- **Security**: `immich.apiKey` and `astrometry.apiKey` are masked on read as `••••` + last 4 characters (empty stays empty). Raw keys never leave the server after being set.
- **Provenance**: API-03, SRC-04, SRC-17. **Confidence**: High.

## SPEC-CFG-2 — Settings document: write

- **Inputs**: `POST /api/admin/settings` with any subset of the sections above.
- **Validation**: if `immich.host` is present it must parse as a URL with `http:`/`https:` scheme → else 400 (`Only HTTP and HTTPS protocols are allowed for Immich host` / `Invalid Immich host URL format`).
- **Behavior**: masked API-key values (starting `••••`) sent back are replaced with the stored real keys before saving (round-tripping the settings form never wipes keys). Sections are persisted per top-level key (a POST containing only `immich` leaves `astrometry` untouched). The in-memory config cache is invalidated so subsequent reads see new values.
- **Outputs**: 200 `{"message":"Settings saved successfully"}`.
- **Side effects / known gap**: changing `immich.syncFrequency` does **not** reschedule the running sync cron until restart (reschedule hooks exist but are not invoked by this route) — see Open Questions Q5.
- **Idempotency**: yes (upsert).
- **Env interplay**: `XMP_SIDECAR_PATH` env var overrides `sidecar.outputPath` regardless of stored value.
- **Provenance**: SRC-04, SRC-17, API-10. **Confidence**: High (validation/masking observed; cron gap source-inferred, Medium).

## SPEC-CFG-3 — Connection tests

- `POST /api/test-astrometry-connection` `{apiKey}`: masked key → use stored; missing → 400 `API key is required`; performs a real login upstream (10s timeout); success → `{success:true, message:"Connection successful!"}`; upstream rejection → 200 `{success:false, message:"Connection failed: …"}`; network errors → 500 with human-readable cause (`Cannot connect…`, `…server not found.`).
- `POST /api/immich/test-immich-connection` `{host, apiKey}`: same masking; both required → 400; URL scheme validated → 400; probes the Immich albums API with the key (10s timeout); non-JSON response → 500 `Server returned non-JSON response (…)`; upstream non-OK → 200 `{success:false, message:"Connection failed with status: N"}`.
- **Provenance**: SRC-04, SRC-05, API-10. **Confidence**: High.

## SPEC-IMG-1 — List images with filters

- **Inputs**: `GET /api/images` with optional `objectType`, `constellation`, `plateSolved` (`true` ⇒ solved, any other value ⇒ unsolved), `equipmentId` (int), repeated `tags` params.
- **Semantics**: all provided filters AND-ed; multiple `tags` values OR-ed (image matches if it has *any* of them); `equipmentId` matches images linked to that equipment.
- **Outputs**: 200 JSON array of complete image records (no pagination, no projection). Empty DB ⇒ `[]`.
- **Compatibility**: gallery, plate-solving page, and sky map all consume this list shape.
- **Provenance**: SRC-03, SRC-21, SRC-35, API-08. **Confidence**: High.

## SPEC-IMG-2 — Read / update one image

- `GET /api/images/:id` → 200 record | 404 `Image not found`. Non-numeric id behaves as not-found (404), not 400 (observed).
- `PATCH /api/images/:id` accepts an arbitrary partial record (no field validation), bumps `updatedAt`, returns the updated record; 404 if missing.
- **Edge cases**: PATCHing derived summary fields works but may be overwritten by the next acquisition/equipment recompute; PATCHing `targetName` immediately affects `/api/targets` grouping (observed).
- **Provenance**: SRC-03, API-08, API-13. **Confidence**: High.

## SPEC-IMG-3 — Image byte serving

- **Inputs**: `GET /api/images/:id/{thumbnail|preview|original}`; optional `?download=1`; `Range` header honored on `original` only; `If-None-Match` honored on all.
- **Outputs**: 200 with body bytes; headers `Content-Type` from file extension (jpg/jpeg/png/gif/webp/tiff mapped; fit/xisf and unknown → `application/octet-stream`), `Cache-Control: public, max-age=31536000, immutable`, weak `ETag: W/"{id}-{size}-{mtimeMs}"`, `Content-Length`, `Accept-Ranges: bytes` (original) / `none` (others). Matching `If-None-Match` (or `*`) → 304 empty. Valid `Range: bytes=a-b` → 206 with `Content-Range: bytes a-b/total`; malformed or out-of-bounds → 416 with `Content-Range: bytes */total`. `?download=1` adds `Content-Disposition: attachment; filename*=UTF-8''…`.
- **Errors**: invalid id → 400 `Invalid image id`; file missing from local storage → 404 `{"message":"Image file not found","backfillPending":true}` with header `X-Sidereal-Backfill: pending` (client contract for "not yet backfilled").
- **Provenance**: SRC-03, API-12, TEST-03. **Confidence**: High.

## SPEC-IMG-4 — Local file upload ingestion

- **Inputs**: `POST /api/sources/upload`, multipart field `file`.
- **Validation**: missing/non-file → 400 `No file provided`; > 500 MB → 413 `File exceeds 500 MB limit`.
- **Behavior**: dedup key = SHA-256(content)[:16]; existing match → return existing image (still 201). Otherwise create image record (title=filename, `objectType:"Deep Sky"`, empty tags, `frameCount:1`), write original + generate preview/thumbnail, set `originalPath`. Any processing failure deletes the record (no orphan rows).
- **Outputs**: 201 `{imageId, filename, sourceType:"local", sourceId}`.
- **Idempotency**: content-idempotent (same bytes → same image id; observed).
- **Provenance**: SRC-14, SRC-29, SRC-22, API-11, FS-01. **Confidence**: High.

## SPEC-IMG-5 — URL ingestion

- **Inputs**: `POST /api/sources/ingest-url` `{url}`.
- **Validation**: missing → 400 `url is required`; unparseable → 400 `Invalid URL`; scheme not http/https → 400 `Only http and https URLs are supported`.
- **Behavior**: dedup key = SHA-256(url)[:16] (checked **before** fetching); fetch; non-OK upstream → error (surfaced as 500 `Failed to ingest image from URL`); extension from Content-Type (`jpeg`→`jpg`, fallback `jpg`); filename from URL basename (fallback `image.jpg`); then identical create/process/rollback flow as local upload.
- **Outputs**: 201 `{imageId, filename, sourceType:"url", sourceId}`.
- **Idempotency**: URL-idempotent (re-ingesting the same URL never re-downloads).
- **Security**: no SSRF guard beyond scheme check (private-network URLs are fetchable) — replicate deliberately or improve; flagged in Open Questions Q7.
- **Provenance**: SRC-14, SRC-30, API-10. **Confidence**: High.

## SPEC-IMG-6 — Processed-image storage contract

- Layout: `{STORAGE_PATH}/processed/{shard}/{id}/` where `shard = zeroPad3(id % 1000)`; files `{id}_original.{ext}` (ext sanitized `[a-z0-9]{1,10}`, default `jpg`), `{id}_preview.jpg`, `{id}_thumb.jpg`.
- Processing constants (explicitly marked as a cross-implementation contract in source): preview max 1920px, thumb max 250px (fit inside, never enlarged), JPEG quality 85/80, EXIF orientation baked, original size cap 500 MB.
- Writes are atomic: staged in `{dir}.tmp` with `.part` files, then swapped in; failure cleans the temp dir. Delete removes the whole `{id}` directory.
- **Provenance**: SRC-22, FS-01, API-11. **Confidence**: High.

## SPEC-SYNC-1 — Immich sync (manual & scheduled)

- **Actors**: user (header button → `POST /api/immich/sync-immich`), scheduler (cron `immich.syncFrequency`).
- **Preconditions**: `immich.host` and `immich.apiKey` configured, else error (`Immich configuration missing. Please configure in admin settings.` — surfaced as generic 500 on the manual route, as failure notification+WS event on the scheduled path).
- **Asset discovery**: if `syncByAlbum`: require non-empty `selectedAlbumIds` (else error `Sync by album is enabled, but no albums are selected.`); fetch each selected album's assets (per-album failures logged and skipped). Otherwise: page Immich metadata search (`type:'IMAGE'`, 1000/page) until exhausted. Duplicate asset ids are de-duplicated.
- **Removal reconciliation**: every Sidereal image with `sourceType:"immich"` whose `sourceId` is no longer present in the fetched asset set is **deleted** (cascade: jobs, links, acquisitions, local files).
- **Ingestion**: for each asset not already known by `(immich, assetId)`: download the original (skip with warning if >500 MB), create the image record with EXIF-derived metadata and defaults (see §4.1), process renditions, set `originalPath`. Per-asset failure ⇒ that asset skipped and any partial record rolled back; sync continues.
- **Outputs**: `{syncedCount, removedCount, message:"Successfully synced N new images from Immich. Removed M images no longer in Immich."}`.
- **Ordering/concurrency**: assets processed sequentially; no lock against concurrent manual+scheduled sync (unknown behavior under overlap — Open Questions Q6).
- **Idempotency**: re-running with unchanged upstream is a no-op (0 synced, 0 removed).
- **Scheduled-run side effects**: WS `source-sync-complete {sourceType:'immich', success, message, syncedCount?, removedCount?}`; on failure an `error` notification "Immich Sync Failed".
- **Provenance**: SRC-28, SRC-19, SRC-05, API-10. **Confidence**: High (source), Medium (live Immich behavior not observed).

## SPEC-SYNC-2 — Original-file backfill

- **Actors**: startup hook; admin (`POST /api/immich/backfill`).
- **Candidates**: images with `sourceType:"immich"`, non-null `sourceId`, whose `originalPath` is missing or outside `{STORAGE_PATH}/processed/`.
- **Behavior**: single-flight (concurrent trigger → 409 `Backfill already running`; startup overlap silently no-ops). Requires Immich config (else error). Processes in batches of 4 concurrent downloads; per-image: download original (skip >500 MB), regenerate renditions, update `originalPath`. Emits WS `backfill-progress {total, processed, failed, skipped}` at start and per batch, and `backfill-complete {processed, failed, skipped, message}` at the end.
- **Outputs (HTTP)**: 200 `{"message":"Backfill started"}` immediately (fire-and-forget).
- **Provenance**: SRC-32, SRC-05, RUN-03 ("Nothing to backfill"), API-10 (409 path not hit but code path confirmed; 200 observed). **Confidence**: High.

## SPEC-SOLVE-1 — Submit an image for plate solving

- **Preconditions**: `astrometry.enabled` && `apiKey` set → else 400 `Plate solving is not configured. Please enable it and provide an API key in the admin settings.`; image exists → else 404; the image's local **preview** rendition must exist (upload material).
- **Behavior**: authenticate with Astrometry.net (api key → session); upload preview JPEG; create a solve job (`status:"processing"`, submission id recorded); broadcast `plate-solving-update {jobId, status:"processing", imageId, message:"Job submitted for plate solving"}`.
- **Two entry styles**: single-image route runs the *full* workflow synchronously (submit → poll up to 720 × pollInterval [~60 min default] → apply results → 200 with `{calibration, annotations, machineTags}`; poll timeout/failure ⇒ 500 `Failed to complete plate solving`). Bulk route only submits and returns per-image `{success, submissionId, jobId}` or `{success:false, error}` (already-solved images rejected with `Image already plate solved`; missing → `Image not found`).
- **Provenance**: SRC-18, SRC-06, API-10. **Confidence**: High (source+error paths observed); upstream interaction Medium (not exercised live).

## SPEC-SOLVE-2 — Applying a successful solve

On success (poll or status check), the system atomically (from the client's perspective):
1. Updates the job: `status:"success"`, `completedAt`, `result` = calibration + annotations.
2. Updates the image: `plateSolved:true`; `ra`/`dec` stringified decimal degrees; `pixelScale`; `fieldOfView` = `(2×radius).toFixed(1) + "'"`; `rotation` = orientation; `astrometryJobId`; `constellation` computed from solved RA/Dec; `tags` = existing ∪ filtered machine tags (dedup).
3. Machine-tag filter (user-visible tag hygiene): drop `The star …` names and malformed fragments; keep catalog designations (M/NGC/IC/Abell/Sh2-/LDN/LBN/Barnard/Cr/Mel/PGC/UGC/Ced/vdB), `astrophotography`, and names containing nebula/cluster/galaxy/supernova/remnant.
4. Auto-assigns `targetName` from the merged tags via catalog match (Messier-priority, then brightest); non-fatal on failure.
5. If sidecars enabled: writes the XMP sidecar (SPEC-SOLVE-4); non-fatal on failure.
6. Broadcasts `plate-solving-update {jobId, status:"success", imageId, result:{…calibration, annotations}}`.
- **Provenance**: SRC-18, SRC-26, SRC-20. **Confidence**: High (source), Medium (end-to-end not run live).

## SPEC-SOLVE-3 — Job status re-check

- **Inputs**: `POST /api/plate-solving/update/:jobId` (also invoked by the worker loop per processing job).
- **Behavior**: query Astrometry.net submission; late-bind the remote job id when it appears; then: remote 404 → mark failed (`Job not found on Astrometry.net. It may have expired (jobs expire after ~30 days).`); remote `failure` → mark failed with explanation (bad hints / image quality); remote `success` or calibration present → fetch full result and apply SPEC-SOLVE-2; still processing → `{status:"processing"}`. Failure results persist diagnostic URLs (`submissionUrl` = nova status page, `jobUrl` = annotated image when known) and broadcast `plate-solving-update {jobId, status:"failed", result}`.
- **Outputs**: `{status, result?}`.
- **Provenance**: SRC-18, SRC-06. **Confidence**: High (source).

## SPEC-SOLVE-4 — XMP sidecar

- **Trigger**: successful solve with `sidecar.enabled`; also fetchable anytime via `GET /api/images/:id/sidecar` (404 `No XMP sidecar found for this image` when absent).
- **Location**: `{outputPath}/{yyyy-mm}/{filename}.xmp` when `organizeByDate` and capture date known, else `{outputPath}/{filename}.xmp`; reader checks date-organized path first, then flat.
- **Content contract** (XML, `application/xml`): XMP metadata containing Dublin Core title (filename) + subject bag of filtered machine tags; a custom astro namespace (`http://ns.astrometry.net/1.0/`) with `plateSolved`, `astrometryJobId`, `plateSolvedAt` ISO timestamp, `imageId` (= `sourceId` when present, else numeric id); calibration block (ra, dec, pixelScale, radius, orientation, parity, optional width/height arcsec); equipment bags grouped by type (name/description/specifications JSON); annotation bag (type, names, pixel coords, radius, ra/dec, magnitude); IPTC keywords mirroring the tags. XML-escaped throughout.
- **Failure mode**: directory create/write failures raise descriptive errors; solve success is not rolled back by sidecar failure.
- **Provenance**: SRC-23, SRC-03. **Confidence**: High (source).

## SPEC-SOLVE-5 — Automatic solving worker

- **Actors**: separate worker process (dev script, container second process, or prod child-process manager).
- **Config**: uses admin settings; *standalone mode* (env-driven: `ASTROMETRY_API_KEY` etc.) activates only when the DB has no key but env does. Config is re-read every loop, so admin changes apply without restart.
- **Loop** every `checkInterval` s (default 30): (a) when `enabled && autoEnabled && apiKey`: submit unsolved images until in-flight (`pending`+`processing`) reaches `maxConcurrent` (default 3); images with an existing job are skipped unless the job failed **and** `autoResubmit` is on; (b) re-check every `processing` job (SPEC-SOLVE-3) and relay non-processing outcomes as `plate-solving-update` frames over its client WS connection to the server. Loop errors back off 5 s.
- **Production supervision**: in-process manager starts the worker only when auto-solving fully configured; crash restarts with exponential backoff (1s→30s, max 5 attempts); status surfaces in `/api/health`. Container script independently restarts a dead worker every 30 s unless `ENABLE_PLATE_SOLVING=false`.
- **Known gap**: worker→server WS frames are not re-broadcast by the server to browser clients (server registers no message handler) — browser updates come from the server-side applies instead. Open Questions Q4.
- **Provenance**: SRC-31, SRC-24, DOCKER-02. **Confidence**: High (source), Medium (loop not observed live).

## SPEC-CAT-1 — Catalog load / update check

- **Load** (`POST /api/catalog/load`, or automatic when empty at startup): download OpenNGC `NGC.csv` + `addendum.csv` (addendum failure tolerated); parse semicolon-CSV with quoted-field support; normalize names (`NGC0001`→`NGC 1`; Messier `M31`→`M 31`); compute decimal degrees from sexagesimal; dedupe by name (NGC wins over addendum); **full replace** (delete all, insert in batches); record `catalog_lastUpdated`, `catalog_commitSha` (latest GitHub commit for NGC.csv), `catalog_objectCount`. Observed result: 14,033 objects. Response `{message:"Loaded N catalog objects", count}`.
- **Check updates** (`POST /api/catalog/check-updates`): `{hasUpdate, currentSha, latestSha}` by comparing stored vs latest GitHub commit; GitHub failure → error.
- **Status** (`GET /api/catalog/status`): `{count, lastUpdated, commitSha}` from stored metadata (zero/nulls before first load).
- **Concurrency note**: a browse during a reload can observe an empty/partial catalog (delete-then-insert, no transaction observed).
- **Provenance**: SRC-20, SRC-39, RUN-04, API-06. **Confidence**: High.

## SPEC-CAT-2 — Catalog browse

- **Inputs**: `GET /api/catalog/browse` with `page` (≥1, default 1), `limit` (1–100 clamped, default 50), `q`, `type` (comma-separated OR), `constellation`, `maxMag`/`minMag` (visual magnitude ≤/≥), `minSize` (major axis ≥ arcmin), `messierOnly=true`, `names` (comma-separated exact names), `latitude` + `hideBelow=true` (exclude objects that never rise for that latitude, hemisphere-aware), `sortBy` ∈ `name|vMag|majorAxis|bestNow`, `sortOrder` ∈ `asc|desc`.
- **Search semantics** (`q`, shared with autocomplete): case-insensitive substring across name, Messier designation, common names, identifiers, for both the raw query and its normalized form (so `m31` finds `NGC 224` via `M 31`).
- **`bestNow` sort** (requires `latitude`): rank by "how close is the object's best month to now", best month derived from RA (object transiting at midnight); never-rising objects last; ties by max transit altitude descending; null-RA last.
- **Non-name sorts** push nulls last.
- **Outputs**: 200 `{items:[catalog objects], total, page, pageSize}` (note: `pageSize`, not `limit`).
- **Provenance**: SRC-20, API-09. **Confidence**: High.

## SPEC-CAT-3 — Catalog object fetch, autocomplete, DSS thumbnail

- `GET /api/catalog/search?q=` → `[]` for empty q; else ≤20 matches (same semantics as browse `q`).
- `GET /api/catalog/:name` (URL-encoded exact name) → object or 404 `Catalog object not found`.
- `GET /api/catalog/thumbnail/:name`: object must exist with coordinates (else 404 `Object not found or has no coordinates`); computes FOV = max(majorAxis/60 × 1.5, 0.05)° (default 0.25° when sizeless); fetches a 300×200 DSS2-color JPEG from the CDS hips2fits service (≤25 concurrent upstream fetches; queued beyond that); upstream failure → 502 `Failed to fetch survey image`; caches to disk under a sanitized filename (`[^a-zA-Z0-9_-]`→`_`, `.jpg`); subsequent requests are served from the cache as static files; response `image/jpeg` with 1-year immutable cache header.
- **Provenance**: SRC-07, API-09. **Confidence**: High (routes observed; upstream fetch source-inferred).

## SPEC-CAT-4 — Tag→target matching & backfill

- **Matcher**: given image tags, normalize each; collect exact catalog-name matches, plus Messier cross-references for `M<n>` tags; order Messier-designated objects first, then ascending visual magnitude; first match wins.
- **Backfill** (`POST /api/catalog/backfill-targets`): for every image with no `targetName` and non-empty tags, run the matcher and set `targetName`; response `{message:"Backfill complete: X images matched, Y already had targets", matched, skipped, total}` (images with a target count as skipped; tagless images count in total only).
- **Provenance**: SRC-20, SRC-07, SRC-39. **Confidence**: High (source).

## SPEC-TGT-1 — Target summaries

- `GET /api/targets` (+ optional `search` on name/common names, `type`, `constellation` filters — case-insensitive substring for search, exact for others), `GET /api/targets/:name`.
- **Grouping**: images grouped by `targetName || title`; each group: `imageCount`, `totalIntegrationHours` (Σ, 3 decimals), `thumbnailImageId` = most recent by capture date, `latestCaptureDate` ISO, `imageIds` (recency-ordered), `objectType`/`constellation` from the matching catalog object when found (by name or Messier) else from the most recent image, `vMag`/`commonNames` from catalog only. Sorted by image count desc, then latest capture desc. 404 `Target not found` for unknown name.
- **Observed example**: single image with `targetName:"NGC 224"` yields a target enriched with `objectType:"G"`, `constellation:"And"`, `vMag:3.44`, `commonNames:"Andromeda Galaxy"`.
- **Provenance**: SRC-11, SRC-38, API-13. **Confidence**: High.

## SPEC-TGT-2 — User target annotations

- `PUT /api/user-targets/:catalogName` `{notes?, tags?}`: upsert keyed by (URL-decoded) catalog name; omitted fields preserved on update; returns the record `{id, catalogName, notes, tags, createdAt, updatedAt}`. `GET` list/single (404 `User target not found`); `DELETE` → `{"message":"User target deleted"}` (idempotent — deleting a nonexistent name still succeeds).
- No validation that the name exists in the catalog.
- **Provenance**: SRC-12, SRC-21, API-13. **Confidence**: High.

## SPEC-EQP-1 — Equipment CRUD

- Create/update require `name` and `type` (else 400 `Name and type are required`); `specifications` defaults `{}`, `description` `""`, `cost`/`acquisitionDate` nullable. PUT is a full replace of those six fields. DELETE → `{"message":"Equipment deleted successfully"}`; deleting equipment cascades away its image links and group memberships and nulls acquisition `filterId`s (dialect-level). **DELETE of a nonexistent id currently returns success** (no existence check) — replicate or fix knowingly (Conflict C8).
- **Provenance**: SRC-08, SRC-21, SCHEMA-01, API-13. **Confidence**: High.

## SPEC-EQP-2 — Image–equipment links

- POST link requires image (404 `Image not found`) and equipment (404 `Equipment not found`); stores optional `settings` object + `notes`; returns the link `{id, imageId, equipmentId, settings, notes, createdAt}`. PUT updates settings/notes for a pair (404 `Equipment relationship not found`). DELETE unlinks (currently always reports success). GET returns equipment records each augmented with the link's `settings`/`notes`.
- **Side effect**: add/remove (not settings-update) triggers image summary recompute (SPEC-EQP-4).
- **Provenance**: SRC-03, SRC-21, API-13. **Confidence**: High.

## SPEC-EQP-3 — Equipment groups

- Create `{name*, description?, memberIds?}` → returns group **with members**; PUT `/:id` renames; PUT `/:id/members {memberIds[]}` replaces membership atomically (array required → 400 `memberIds must be an array`); DELETE removes group + memberships; GET list/single include `members` arrays. Apply-to-image adds only members not already linked, returns `{added:[equipment...], message:"N equipment item(s) assigned"}` (re-apply → 0 assigned; idempotent) and recomputes the image summary when anything was added.
- **Provenance**: SRC-09, SRC-21. **Confidence**: High (source; apply not run live).

## SPEC-EQP-4 — Acquisition entries & derived image summary

- CRUD under `/api/images/:id/acquisitions`; create/update require `frameCount` and `exposureTime` (create → 400 `frameCount and exposureTime are required`); optional `filterId|filterName, gain, offset, binning, sensorTemp, date, notes`. Delete of unknown id → 404 `Acquisition entry not found`.
- **Recompute contract** (observed): after any acquisition change or equipment link change, the image's summary fields are recomputed: `frameCount` = Σ frameCount; `totalIntegration` = Σ(frames×exposure)/3600 (3-decimal rounding); `filters` = comma+space-joined distinct non-null filter names (null when none); `exposureTime` = `"{t}s"` if all equal else `"{min}s - {max}s"`; when a linked telescope has `focalLength`/`focalRatio` specs, image `focalLength` (number) and `aperture` (the focal *ratio* string, e.g. `f/6`) are set. Observed: 10×300s Ha + 5×120s OIII + telescope(600, f/6) ⇒ `{frameCount:15, totalIntegration:1, filters:"Ha, OIII", exposureTime:"120s - 300s", focalLength:600, aperture:"f/6"}`.
- **Provenance**: SRC-03, SRC-21, SRC-38, API-13. **Confidence**: High.

## SPEC-LOC-1 — Locations

- CRUD at `/api/locations`; create takes `{name, latitude, longitude, altitude?, description?}` and returns the record; PATCH partial-updates (404 when missing); DELETE → `{"message":"Location deleted"}` (idempotent). No observed server-side validation of coordinate ranges and no server-side link to images.
- **Provenance**: SRC-10, API-13. **Confidence**: High.

## SPEC-NTF-1 — Notifications

- Created by the system on background failures (`error` type observed: sync failure, cron scheduling/unhandled errors) with `{type,title,message,details}`.
- `GET /api/notifications` returns **only unacknowledged** ones. Acknowledge single (`POST /:id/acknowledge`) or all; both return success messages and broadcast WS `notifications-updated {}`. Nightly job deletes acknowledged notifications older than 30 days.
- UI contract (test-asserted): header bell shows count; admin page card lists first 5 with Show all/Show less toggle; "Acknowledge All" only with >1; card hidden when none.
- **Provenance**: SRC-19, SRC-04, SRC-21, TEST-09. **Confidence**: High.

## SPEC-STAT-1 — Stats, tags, constellations

- `GET /api/stats` → `{totalImages, plateSolved, totalHours (Σ totalIntegration, 2-dec), uniqueTargets (distinct non-empty titles), totalEquipment, objectTypeCounts {type|"Unknown": n}, plateSolvingStats {total, pending, successful, failed}}` (exact field names observed).
- `GET /api/tags` → top 20 `{tag, count}` by frequency across images, counting only "relevant" tags (same filter as SPEC-SOLVE-2 step 3).
- `GET /api/constellations` → sorted distinct non-null constellation values across images.
- **Provenance**: API-02, SRC-04, SRC-26. **Confidence**: High.

## SPEC-ADM-1 — Database info & backup

- `GET /api/admin/database` → `{type:"sqlite"|"postgresql"}`; for SQLite adds `path`, `sizeBytes`, `lastModified` (ISO; `sizeBytes:0` when file missing).
- `GET /api/admin/database/backup` → SQLite file streamed as `application/x-sqlite3` attachment named `sidereal-backup-{ISO-ish timestamp}.db`; PostgreSQL → 400 `Backup download is only available for SQLite databases. Use pg_dump for PostgreSQL.`
- **Provenance**: SRC-04, API-05. **Confidence**: High.

## SPEC-ADM-2 — Cron job status

- `GET /api/admin/cron-jobs` is *intended* to return job list (id, name, schedule, lastRun, lastError, enabled) but **currently fails with 500 `Failed to get cron jobs`** because live scheduler handles are not JSON-serializable. A replacement should return the serializable fields. Documented in docs; broken at runtime (Conflict C1).
- **Provenance**: SRC-04, SRC-19, API-04, DOC-01. **Confidence**: High (failure observed).

## SPEC-PRX-1 — Immich asset proxy

- `GET /api/assets/:assetId/:type` with `type ∈ {thumbnail, original, fullsize}`; asset id must match `[a-zA-Z0-9-]+` (else 400 `Invalid asset ID format`); unknown type → 400 `Invalid asset type`; Immich unconfigured → **503** `Immich configuration missing`; stored host scheme re-validated (SSRF guard); query string forwarded (e.g. `?size=preview`); upstream status forwarded on failure; success streams the body with upstream content-type. Note: the current frontend no longer uses this for gallery rendering (TEST-03 asserts zero `/api/assets/` img sources) — it remains a public compatibility surface.
- **Provenance**: SRC-15, TEST-03. **Confidence**: High (source), Medium (upstream not exercised).

## SPEC-WS-1 — Real-time updates

- Endpoint `/{ws}` on the HTTP port; JSON envelope `{event, data}`; broadcast to every connected client; no auth, no rooms, no acks, no server-consumed inbound messages. Event catalog in §3.3. Clients reconnect with 1s→30s exponential backoff and use events purely as refetch triggers.
- **Provenance**: SRC-25, SRC-34, WS-01. **Confidence**: High.

## SPEC-JOB-1 — Scheduled maintenance jobs

- **Notification cleanup** daily 02:00: delete acknowledged notifications older than 30 days.
- **Orphan sweep** daily 03:00: walk `{STORAGE_PATH}/processed/*/*`; any image directory whose numeric id has no DB record is removed recursively; count logged.
- Cron task errors are caught, logged, and create an `error` notification ("Cron Job Error").
- **Provenance**: SRC-19, SRC-21, SRC-22, RUN-03. **Confidence**: High (registration observed; execution source-inferred).

## SPEC-DCK-1 — Container runtime

- Image exposes 5000; healthcheck hits `/api/health`; runtime dirs `/app/{config,logs,sidecars,cache/thumbnails,data/images/processed}` pre-created; runs as non-root `sidereal` user after PUID/PGID remap (defaults 1001).
- Entrypoint behavior: PG readiness wait (60 s, fatal timeout) when `DATABASE_URL` set; PG migrations best-effort; optional one-time data migration when `AUTO_DB_MIGRATE_FROM` set (`--from`/`--to` a second DB URL or `sqlite:` path; marker file prevents re-run when `AUTO_DB_MIGRATE_ONCE=true`; existing SQLite target removed first by default; failure aborts startup); then main + worker processes started and supervised (worker auto-restart; main death kills container; SIGTERM graceful).
- Compose (SQLite default): named volumes for config/logs/sidecars/cache/images + **external** `sidereal_immich-upload` volume; `SIDEREAL_PORT` maps external port. PG override adds a `postgres:15-alpine` service (`sidereal`/`sidereal` db/user) and injects `DATABASE_URL`.
- **Provenance**: DOCKER-01..04, DOC-03. **Confidence**: High (artifacts), Medium (container not run).
