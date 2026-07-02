# 8. Worker and Integration Semantics

What users and API/WS clients can observe of background work and external integrations. Full per-behavior detail lives in §5 (SPEC-SYNC-*, SPEC-SOLVE-*, SPEC-JOB-*, SPEC-CAT-1); this section is the observability contract.

## 8.1 Immich sync

| Observable | Behavior |
|---|---|
| Trigger | Header button / `POST /api/immich/sync-immich` (synchronous, returns counts) · cron per `immich.syncFrequency` (default every 4 h) |
| Success signal | HTTP `{syncedCount, removedCount, message}`; scheduled runs also broadcast WS `source-sync-complete {sourceType:'immich', success:true, message, syncedCount, removedCount}` |
| Failure signal | Manual: 500 `{"message":"Failed to sync with Immich"}` (details in server log). Scheduled: WS `source-sync-complete {success:false, message}` **plus** persisted `error` notification "Immich Sync Failed" with details `{jobId:'immich-sync', timestamp, error}` |
| Persisted state | New image records (+ local original/preview/thumbnail files); deletions of images whose Immich assets vanished (cascade incl. files, jobs, links, acquisitions) |
| Partial failure | Per-asset: oversized (>500 MB) or failed downloads are skipped with a log warning and any partial record rolled back; sync continues and still reports success counts. Per-album fetch failures are skipped. |
| Idempotency | Re-sync with unchanged upstream: 0 synced / 0 removed |
| Timeouts/retries | No retry policy; connection tests use 10 s timeouts; sync fetches have no explicit timeout (Unknown behavior under hang — Q6) |

Provenance: SRC-28, SRC-19, SRC-05, API-10. Confidence: High (source/API), Medium (live Immich unobserved).

## 8.2 Astrometry.net plate solving

| Observable | Behavior |
|---|---|
| Manual trigger (single) | `POST /api/plate-solving/images/{id}/plate-solve` blocks until solved (poll every `pollInterval` s, up to 720 attempts ≈ 60 min at default) and returns calibration+annotations+machineTags; failure/timeout → 500 |
| Manual trigger (bulk) | `POST /api/plate-solving/bulk` returns immediately with per-image submission results; progress arrives via WS + job list polling |
| Automatic | Worker loop (see §5 SPEC-SOLVE-5): auto-submits unsolved images when `enabled && autoEnabled && apiKey`, caps in-flight at `maxConcurrent`, honors `autoResubmit` for failed jobs, re-checks processing jobs every `checkInterval` s |
| Job records | `GET /api/plate-solving/jobs`: `processing` → `success` (result = calibration + annotations) or `failed` (result = `{error, submissionId, astrometryJobId, submissionUrl, jobUrl}` with nova.astrometry.net links) |
| WS events | `plate-solving-update` at submission (`processing` + message), success (with result), failure (with error result) |
| Image effects on success | `plateSolved`, RA/Dec, pixel scale, FOV string, rotation, constellation, merged relevant tags, auto `targetName`, XMP sidecar file (when enabled) |
| Upload material | The locally stored **preview** JPEG (≤1920 px), not the original — a compatibility-relevant behavior (solve accuracy/scale hints derive from the preview) |
| Failure taxonomy surfaced | "could not solve" (hints/quality), "job expired (~30 days)", generic processing failure; all non-retryable automatically unless `autoResubmit` |
| Health/ops | `/api/health.worker` `{enabled, running, pid, restartAttempts}` (production in-process manager); container separately restarts a dead worker process every 30 s |

Provenance: SRC-18, SRC-31, SRC-24, SRC-06, DOCKER-02, API-01. Confidence: High (source), Medium (upstream lifecycle unobserved).

## 8.3 Scheduled jobs

| Job | Schedule | Observables |
|---|---|---|
| Immich sync | `immich.syncFrequency` (default `0 */4 * * *`) | WS event, notifications, image changes; log lines `[CRON] Starting/completed Immich sync` |
| Notification cleanup | `0 2 * * *` | acknowledged notifications >30 days old disappear; log `[CRON] Old notifications cleaned up` |
| Orphan image sweep | `0 3 * * *` | on-disk `processed/{shard}/{id}` dirs without DB records deleted; log `[ORPHAN-SWEEP] Removed orphan dir for id=…` / `Done: N orphans removed` |

Registration is logged at startup (`[CRON] Scheduled … with cron: …` — observed). Any cron task error is caught, logged, and produces an `error` notification ("Cron Job Error"). The status endpoint for these jobs currently 500s (C1). Changing the sync schedule via settings does not take effect until restart (Q5). Provenance: SRC-19, RUN-03, API-04.

## 8.4 Image backfill (Immich originals)

- Triggers: every startup (silent no-op when nothing pending — observed log `[BACKFILL] Nothing to backfill`) and `POST /api/immich/backfill` (202-like: 200 "Backfill started"; 409 when already running).
- Progress: WS `backfill-progress {total, processed, failed, skipped}` emitted at start and after each 4-image batch; terminal `backfill-complete {processed, failed, skipped, message}`.
- Persisted effect: image `originalPath` updated to the local processed path; renditions regenerated.
- Byte-serving interplay: until backfilled, `/api/images/{id}/*` returns the 404+`backfillPending` contract so the UI can distinguish "not yet local" from "gone".
- Partial failures: per-image failures counted (`failed`) without stopping the run; >500 MB originals counted as `skipped` permanently (retried only on a later run; skip set is in-memory per run).

Provenance: SRC-32, RUN-03, API-10, API-12. Confidence: High.

## 8.5 Catalog load

- Automatic on startup when empty (observed: fetch + "Loaded 14033 catalog objects" while the server is already accepting requests — load is non-blocking), or manual via admin.
- Full-replace semantics: during a reload, browse/search may briefly observe an empty or partial catalog. Metadata (`count`, `lastUpdated`, `commitSha`) updates only at the end.
- Failure: logged (`Failed to auto-load catalog`), server keeps running with whatever catalog exists (possibly none). Manual load surface errors as 500.

Provenance: SRC-01, SRC-20, RUN-04. Confidence: High.

## 8.6 DSS thumbnail fetching

- On-demand per catalog object; first request fetches from the CDS survey service (max 25 concurrent upstream fetches, extra requests queue), caches to disk, then all subsequent requests are static-file hits. Upstream failure → 502; nothing cached. Provenance: SRC-07. Confidence: High (source).

## 8.7 Cross-process event topology (as-built)

- Server broadcasts to all browser WS clients.
- The worker process connects to the server as a *client* (`ws://localhost:5000/ws`) with 1s→30s reconnect backoff and *sends* `plate-solving-update` frames — but the server has no inbound-message handler, so these frames are not relayed to browsers (Q4). Browser-visible updates originate from the server-side apply path (`updateJobAndImage`) and route-level broadcasts. A replacement implementation only needs to preserve the *browser-observable* events listed in §3.3.

Provenance: SRC-31, SRC-25. Confidence: High (source).
