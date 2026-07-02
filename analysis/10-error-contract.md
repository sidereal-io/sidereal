# 10. Error Contract

## 10.1 Universal response shape

Every JSON error is `{"message": "<human-readable string>"}` — no error codes, no field-level detail, no `error` sub-object (docs show an `error` field that does not exist — C6). The byte-serving 404 uniquely extends it: `{"message":"Image file not found","backfillPending":true}` + header `X-Sidereal-Backfill: pending`. (API-04..12, SRC-16)

Route handlers wrap failures generically: unexpected exceptions become 500 with a fixed context message (e.g. `Failed to fetch images`, `Failed to sync with Immich`); when an upstream HTTP client error object carries a status, that status and message are forwarded instead. Details go to server logs only. (SRC-16)

## 10.2 Validation errors (400 unless noted) — observed messages

| Case | Message |
|---|---|
| Settings: non-http(s)/unparseable Immich host | `Only HTTP and HTTPS protocols are allowed for Immich host` / `Invalid Immich host URL format` |
| Immich test/albums: missing host or key | `Host and API key are required` |
| Immich test/albums: bad URL | `Only HTTP and HTTPS protocols are allowed` / `Invalid URL format` |
| Astrometry test: missing key | `API key is required` |
| Plate solve: feature off/unconfigured | `Plate solving is not configured. Please enable it and provide an API key in the admin settings.` |
| Bulk solve: bad list | `imageIds array is required` |
| Upload: no file | `No file provided`; oversized → **413** `File exceeds 500 MB limit` |
| URL ingest: missing/invalid/wrong scheme | `url is required` / `Invalid URL` / `Only http and https URLs are supported` |
| Equipment create/update | `Name and type are required` |
| Group members: non-array | `memberIds must be an array` |
| Group create: no name | `Name is required` |
| Acquisition create | `frameCount and exposureTime are required` |
| Byte serving: NaN id | `Invalid image id` |
| Asset proxy: bad id/type | `Invalid asset ID format` / `Invalid asset type` |
| Solve-data reads on unsolved image | `Image has not been (successfully) plate solved` / `No successful plate solving data found` |

Notable *absences* (replicate or knowingly improve): PATCH image body is unvalidated; JSON type errors mostly become generic 500s; malformed JSON bodies produce 500-family generic messages; non-numeric path ids on plain GETs behave as 404 not 400 (API-08).

## 10.3 Missing configuration

| Surface | Behavior |
|---|---|
| Manual Immich sync, unconfigured | 500 `Failed to sync with Immich` (specific reason only in logs/scheduled-path notification) — observed |
| Scheduled sync, unconfigured/failing | WS `source-sync-complete {success:false}` + persisted error notification |
| Asset proxy, unconfigured | **503** `Immich configuration missing` |
| Backfill, unconfigured | run aborts; error logged (manual trigger already returned 200 "Backfill started") |
| Plate solving disabled/no key | 400 (message above) |
| Worker with nothing configured | idles, logging "Automatic plate solving not enabled or configured" |

## 10.4 Not-found behavior (404)

`Image not found`, `Equipment not found`, `Equipment relationship not found`, `Acquisition entry not found`, `Equipment group not found`, `Location not found`, `Target not found`, `Catalog object not found`, `User target not found`, `Unknown source type: {type}`, `No XMP sidecar found for this image`, `Object not found or has no coordinates` (DSS), `Image file not found` (+backfill marker), unmatched API path → `Not found`. Inconsistency to be aware of: several DELETEs (equipment, group, location, user-target, image-equipment) return success for nonexistent targets (idempotent-style), while others (acquisition) 404 (C8).

## 10.5 External-service failures

| Dependency | Failure surfaced as |
|---|---|
| Immich unreachable (tests) | 500 with cause-specific text: `Cannot connect to Immich server. Please check the host URL.` (ECONNREFUSED), `Host not found. Please check the host URL.` (ENOTFOUND), or raw error message; non-JSON response → 500 `Server returned non-JSON response (…)` |
| Immich rejects key (test) | 200 `{success:false, message:"Connection failed with status: N"}` (note: *200*, failure in body) |
| Astrometry login rejected (test) | 200 `{success:false, message:"Connection failed: …"}`; network errors → 500 with cause text |
| Astrometry solve failures | Job → `failed` with persisted explanation + diagnostic URLs; WS event; no automatic retry (unless `autoResubmit`) |
| Astrometry poll errors | Retried silently until attempt cap; timeout → workflow error (500 on the synchronous route) |
| GitHub (catalog) down | Manual load/check → 500 (`GitHub API returned N` etc.); startup auto-load logs and continues |
| DSS survey down | 502 `Failed to fetch survey image` |
| Immich proxy upstream error | upstream status + statusText forwarded |

## 10.6 Permission/auth failures

None exist: there is no authentication, no authorization, no per-user data, no CSRF protection, and no rate limiting. Every endpoint (including settings write, DB backup download, and catalog reload) is anonymous. A replacement must treat this as the compatibility baseline **and** as a deployment assumption (trusted network). API keys are the only secrets and are masked on read. (DOC-01 confirms "no authentication" as intended; API observations confirm.)

## 10.7 Conflict / concurrency cases

| Case | Behavior |
|---|---|
| Backfill started while running | **409** `Backfill already running` — the only 409 in the app |
| Duplicate upload (same content/URL) | Not an error: 201 with the existing image (idempotent ingest) |
| Bulk solve incl. solved image | Per-item `{success:false, error:"Image already plate solved"}` inside a 200 |
| Concurrent syncs / catalog reload during browse / simultaneous same-image writes | No locking observed; outcomes undefined (Q6, U4) |

## 10.8 Retryable vs non-retryable (client guidance, as-built)

- Retryable by client: 500s from transient upstream failures (sync, tests, catalog ops), 502 DSS, 503 proxy-unconfigured (after fixing config), 409 backfill (later).
- Non-retryable without change: all 400s, 404s, 413.
- The frontend itself retries nothing (queries and mutations configured no-retry) and relies on WS-triggered refetch. (SRC-35)

## 10.9 UI presentation

- API failures on data loads leave sections in empty/fallback states; page shells always render (TEST-04).
- Mutations surface toast notifications with the server `message` (equipment, solve submission, settings save, connection tests).
- Background failures surface as persistent notifications (bell + admin card) rather than toasts.
- The gallery treats the `backfillPending` 404 as "image not ready" rather than broken (dedicated header/body marker).
