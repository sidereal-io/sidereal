# 6. API Contract Summary

Notation: JSON-like schemas; `?` = optional/nullable; `Image`, `Job`, `Equipment`, `CatalogObject`, etc. defined once and reused. All endpoints are unauthenticated. All error bodies are `{message: string}` unless noted. Status codes listed are the ones the current implementation can actually produce. Confidence is High where marked ✦ (live-observed) and Medium otherwise (source-derived).

## Shared shapes (observed on the wire ✦)

```ts
Image = {
  id: int, sourceType: "immich"|"local"|"url", sourceId: string?,
  title: string, filename: string, originalPath: string?,
  captureDate: iso8601?, focalLength: number?, aperture: string?, iso: int?,
  exposureTime: string?, frameCount: int?, totalIntegration: number?,   // hours
  telescope: string?, camera: string?, mount: string?, filters: string?,
  latitude: number?, longitude: number?, altitude: number?,
  plateSolved: boolean, ra: string?, dec: string?, pixelScale: number?,
  fieldOfView: string?, rotation: number?, astrometryJobId: string?,
  tags: string[]?, objectType: string?, constellation: string?, targetName: string?,
  description: string?, createdAt: iso8601, updatedAt: iso8601
}
Equipment = { id: int, name: string, type: string, specifications: object?,
  imageUrl: string?, description: string?, cost: number?, acquisitionDate: iso8601?,
  createdAt: iso8601, updatedAt: iso8601 }
ImageEquipmentLink = { id: int, imageId: int, equipmentId: int,
  settings: object?, notes: string?, createdAt: iso8601 }
Acquisition = { id: int, imageId: int, filterId: int?, filterName: string?,
  frameCount: int, exposureTime: number, gain: int?, offset: int?, binning: string?,
  sensorTemp: number?, date: iso8601?, notes: string?, createdAt: iso8601 }
Job = { id: int, imageId: int?, astrometrySubmissionId: string?, astrometryJobId: string?,
  status: "pending"|"processing"|"success"|"failed",
  submittedAt: iso8601, completedAt: iso8601?, result: object? }
CatalogObject = { id: int, name: string, type: string?, ra: string?, dec: string?,
  raDeg: number?, decDeg: number?, constellation: string?, majorAxis: number?,
  minorAxis: number?, bMag: number?, vMag: number?, surfaceBrightness: number?,
  hubbleType: string?, messier: string?, ngcRef: string?, icRef: string?,
  commonNames: string?, identifiers: string?, createdAt: iso8601 }
TargetSummary = { targetName: string, imageCount: int, totalIntegrationHours: number,
  thumbnailImageId: int?, objectType: string?, constellation: string?, vMag: number?,
  commonNames: string?, latestCaptureDate: iso8601?, imageIds: int[] }
UserTarget = { id: int, catalogName: string, notes: string?, tags: string[]?,
  createdAt: iso8601, updatedAt: iso8601 }
Location = { id: int, name: string, latitude: number, longitude: number,
  altitude: number?, description: string?, createdAt: iso8601, updatedAt: iso8601 }
Notification = { id: int, type: "error"|"warning"|"info"|"success", title: string,
  message: string, details: object?, acknowledged: boolean, createdAt: iso8601 }
SourceStatus = { displayName?: string, sourceType: string, lastSync: iso8601?,
  imageCount: int, healthy: boolean, message?: string }
IngestResult = { imageId: int, filename: string, sourceType: string, sourceId: string }
```

## Images

| Endpoint | Request | Response | Codes | Notes | Prov |
|---|---|---|---|---|---|
| GET `/api/images` ✦ | query: `objectType?, constellation?, plateSolved?("true"/other), equipmentId?, tags?*` | `Image[]` | 200, 500 | filters AND-ed; tags OR-ed | SRC-03, API-08 |
| GET `/api/images/{id}` ✦ | — | `Image` | 200, 404, 500 | non-numeric id → 404 | API-08 |
| PATCH `/api/images/{id}` ✦ | partial `Image` (unvalidated) | updated `Image` | 200, 404, 500 | bumps `updatedAt`; UI uses it for title/description/target/tags/coords edits | SRC-03, API-13 |
| GET `/api/images/{id}/thumbnail\|preview\|original` ✦ | headers `If-None-Match?`, `Range?` (original); query `download=1?` | bytes | 200, 206, 304, 400, 404, 416 | see SPEC-IMG-3 for headers; 404 body adds `backfillPending:true` + `X-Sidereal-Backfill: pending` | API-12 |
| GET `/api/images/{id}/plate-solving-job` | — | `{jobId,submissionId,astrometryJobId,status,submittedAt,completedAt}` | 200, 400, 404, 500 | 400 `Image has not been successfully plate solved` | SRC-03 |
| GET `/api/images/{id}/annotations` | — | `{annotations: [], calibration: {ra,dec,pixscale,radius,orientation}, imageDimensions: {width?,height?}}` | 200, 400, 404, 500 | from stored job result | SRC-03 |
| GET `/api/images/{id}/equipment` ✦ | — | `(Equipment & {settings,notes})[]` | 200, 404, 500 | | API-13 |
| POST `/api/images/{id}/equipment` ✦ | `{equipmentId, settings?, notes?}` | `ImageEquipmentLink` | 200, 404, 500 | 404 for missing image or equipment; recomputes summary | API-13 |
| PUT `/api/images/{imageId}/equipment/{equipmentId}` | `{settings?, notes?}` | `ImageEquipmentLink` | 200, 404, 500 | | SRC-03 |
| DELETE `/api/images/{imageId}/equipment/{equipmentId}` | — | `{message}` | 200, 500 | always succeeds (no existence check); recomputes | SRC-03, SRC-21 |
| GET `/api/images/{id}/acquisitions` | — | `Acquisition[]` | 200, 500 | | SRC-03 |
| POST `/api/images/{id}/acquisitions` ✦ | `{frameCount*, exposureTime*, filterId?, filterName?, gain?, offset?, binning?, sensorTemp?, date?, notes?}` | `Acquisition` | 200, 400, 500 | recomputes summary | API-13 |
| PUT `/api/images/{imageId}/acquisitions/{id}` | same body | `Acquisition` | 200, 404, 500 | recomputes | SRC-03 |
| DELETE `/api/images/{imageId}/acquisitions/{id}` | — | `{message}` | 200, 404, 500 | recomputes | SRC-03 |
| GET `/api/images/{id}/sidecar` | query `download=true?` | XMP XML (`application/xml`) | 200, 404, 500 | attachment `{filename}.xmp` on download | SRC-03 |

## Plate solving

| Endpoint | Request | Response | Codes | Notes | Prov |
|---|---|---|---|---|---|
| POST `/api/plate-solving/images/{id}/plate-solve` ✦(400) | — | `{message, result: {calibration, annotations, machineTags}}` | 200, 400, 404, 500 | synchronous full workflow; 400 when disabled/unconfigured | SRC-06, API-10 |
| GET `/api/plate-solving/jobs` | — | `Job[]` | 200, 500 | | SRC-06 |
| POST `/api/plate-solving/bulk` ✦(400) | `{imageIds: int[]}` | `{message, results: [{imageId, success, submissionId?, jobId?, message?, error?}]}` | 200, 400, 500 | empty/missing array → 400 `imageIds array is required` | SRC-06, API-10 |
| POST `/api/plate-solving/update/{jobId}` | — | `{status, result?}` | 200, 500 | re-checks upstream; broadcasts WS | SRC-06 |

## Sources & ingestion

| Endpoint | Request | Response | Codes | Notes | Prov |
|---|---|---|---|---|---|
| GET `/api/sources` ✦ | — | `SourceStatus[]` (with `displayName`) | 200, 500 | three sources: local, url, immich | API-07 |
| POST `/api/sources/upload` ✦ | multipart `file` | `IngestResult` | 201, 400, 413, 500 | content-hash dedup (still 201) | API-11 |
| POST `/api/sources/ingest-url` ✦(400) | `{url}` | `IngestResult` | 201, 400, 500 | URL-hash dedup | API-10 |
| GET `/api/sources/{type}/status` ✦(404) | — | `SourceStatus` (no displayName) | 200, 404, 500 | 404 `Unknown source type: X` | API-10 |
| POST `/api/sources/{type}/test` | — | `{ok: boolean, message?}` | 200, 404, 500 | | SRC-14 |

## Immich integration

| Endpoint | Request | Response | Codes | Notes | Prov |
|---|---|---|---|---|---|
| POST `/api/immich/sync-immich` ✦(500) | — | `{syncedCount, removedCount, message}` | 200, 500, 503 | 503 only if source plugin missing; unconfigured → 500 generic | SRC-05, API-10 |
| POST `/api/immich/test-immich-connection` ✦(400) | `{host, apiKey}` (masked key allowed) | `{success, message}` | 200, 400, 500 | see SPEC-CFG-3 | API-10 |
| POST `/api/immich/albums` | `{host, apiKey}` (masked allowed) | `[{id, albumName}]` | 200, 400, 500 | | SRC-05 |
| POST `/api/immich/backfill` ✦ | — | `{message:"Backfill started"}` | 200, 409, 500 | fire-and-forget; 409 when running | API-10 |
| GET `/api/assets/{assetId}/{type}` | query forwarded | image bytes (upstream content-type) | 200, 400, 5xx-forwarded, 500, 503 | `type ∈ thumbnail\|original\|fullsize`; 503 unconfigured | SRC-15 |

## Catalog & targets

| Endpoint | Request | Response | Codes | Notes | Prov |
|---|---|---|---|---|---|
| GET `/api/catalog/browse` ✦ | query per SPEC-CAT-2 | `{items: CatalogObject[], total, page, pageSize}` | 200, 500 | `pageSize` (docs wrongly say `limit` — C3) | API-09 |
| GET `/api/catalog/search?q=` ✦ | `q` (empty → `[]`) | `CatalogObject[]` ≤20 | 200, 500 | | API-09 |
| GET `/api/catalog/status` ✦ | — | `{count, lastUpdated, commitSha}` | 200, 500 | docs wrongly say `objectCount` — C3 | API-06 |
| GET `/api/catalog/thumbnail/{name}` | — | JPEG | 200, 404, 502, 500 | disk-cached | SRC-07 |
| GET `/api/catalog/{name}` ✦ | URL-encoded name | `CatalogObject` | 200, 404, 500 | exact match | API-09 |
| POST `/api/catalog/load` | — | `{message, count}` | 200, 500 | full replace from GitHub | SRC-20, RUN-04 |
| POST `/api/catalog/check-updates` | — | `{hasUpdate, currentSha, latestSha}` | 200, 500 | | SRC-20 |
| POST `/api/catalog/backfill-targets` | — | `{message, matched, skipped, total}` | 200, 500 | | SRC-07 |
| GET `/api/targets` ✦ | `search?, type?, constellation?` | `TargetSummary[]` | 200, 500 | | API-13 |
| GET `/api/targets/{name}` | — | `TargetSummary` | 200, 404, 500 | | SRC-11 |
| GET `/api/user-targets` | — | `UserTarget[]` | 200, 500 | | SRC-12 |
| GET `/api/user-targets/{catalogName}` | — | `UserTarget` | 200, 404, 500 | | SRC-12 |
| PUT `/api/user-targets/{catalogName}` ✦ | `{notes?, tags?}` | `UserTarget` | 200, 500 | upsert | API-13 |
| DELETE `/api/user-targets/{catalogName}` | — | `{message}` | 200, 500 | idempotent | SRC-12 |

## Equipment

| Endpoint | Request | Response | Codes | Notes | Prov |
|---|---|---|---|---|---|
| GET `/api/equipment` | — | `Equipment[]` | 200, 500 | | SRC-08 |
| POST `/api/equipment` ✦ | `{name*, type*, specifications?, description?, cost?, acquisitionDate?}` | `Equipment` | 200, 400, 500 | | API-13 |
| PUT `/api/equipment/{id}` | same | `Equipment` | 200, 400, 404, 500 | full replace of the six fields | SRC-08 |
| DELETE `/api/equipment/{id}` | — | `{message}` | 200, 500 | success even when id unknown (C8) | SRC-08, SRC-21 |
| GET `/api/equipment-groups[/{id}]` | — | `(Group & {members: Equipment[]})[]` / one | 200, 404, 500 | | SRC-09 |
| POST `/api/equipment-groups` | `{name*, description?, memberIds?}` | group w/ members | 200, 400, 500 | | SRC-09 |
| PUT `/api/equipment-groups/{id}` | `{name?, description?}` | group w/ members | 200, 404, 500 | | SRC-09 |
| PUT `/api/equipment-groups/{id}/members` | `{memberIds: int[]}` | group w/ members | 200, 400, 500 | replace-all | SRC-09 |
| DELETE `/api/equipment-groups/{id}` | — | `{message}` | 200, 500 | | SRC-09 |
| POST `/api/equipment-groups/{id}/apply/{imageId}` | — | `{added: Equipment[], message}` | 200, 500 | idempotent add-missing | SRC-09 |

## System / admin / misc

| Endpoint | Request | Response | Codes | Notes | Prov |
|---|---|---|---|---|---|
| GET `/api/admin/settings` ✦ | — | settings doc (keys masked) | 200, 500 | SPEC-CFG-1 | API-03 |
| POST `/api/admin/settings` ✦(400) | settings doc subset | `{message}` | 200, 400, 500 | SPEC-CFG-2 | API-10 |
| POST `/api/test-astrometry-connection` ✦(400) | `{apiKey}` | `{success?, message}` | 200, 400, 500 | | API-10 |
| GET `/api/stats` ✦ | — | see SPEC-STAT-1 | 200, 500 | field names differ from docs (C4) | API-02 |
| GET `/api/tags` | — | `[{tag, count}]` ≤20 | 200, 500 | | SRC-04 |
| GET `/api/constellations` | — | `string[]` sorted | 200, 500 | | SRC-04 |
| GET `/api/notifications` ✦ | — | `Notification[]` (unacknowledged) | 200, 500 | | SRC-04 |
| POST `/api/notifications/{id}/acknowledge` | — | `{message}` | 200, 500 | WS `notifications-updated` | SRC-04 |
| POST `/api/notifications/acknowledge-all` | — | `{message}` | 200, 500 | WS `notifications-updated` | SRC-04 |
| GET `/api/admin/cron-jobs` ✦ | — | *(intended: job list)* | **500 today** | broken — C1 | API-04 |
| GET `/api/admin/database` ✦ | — | `{type, path?, sizeBytes?, lastModified?}` | 200, 500 | | API-05 |
| GET `/api/admin/database/backup` | — | sqlite bytes (`application/x-sqlite3`, attachment) | 200, 400, 500 | 400 on PG | SRC-04 |
| GET `/api/health` ✦ | — | health doc | 200, 503 | SPEC-SYS-3 | API-01 |
| GET `/api/<unmatched>` ✦ | — | `{message:"Not found"}` | 404 | | API-08 |

## Endpoints documented but NOT present (docs/api.md drift — see Conflict Log)

- `POST /api/immich/sync-metadata/{imageId}` and `POST /api/immich/sync-metadata-all` — **do not exist** in the current router (C2). Settings still carry `metadataSyncEnabled`/`syncDescription`/`syncCoordinates`/`syncTags` flags with no observed consumer (Q3).
- Image responses no longer include `immichId`/`thumbnailUrl`/`fullUrl` fields shown in docs (C5); clients derive URLs as `/api/images/{id}/{size}`.
