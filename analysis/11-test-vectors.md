# 11. Test Vectors

Black-box, implementation-independent. Priority: P0 = compatibility-critical, P1 = important, P2 = nice-to-verify. "Fresh instance" = empty database + empty storage dir + default settings. Vectors marked ✔ were executed against the current implementation during this analysis (provenance cited); others are derived from confirmed behavior and existing E2E assertions.

## Startup, health, migration

| ID | Spec | Setup | Action | Expected | Prov | Pri |
|---|---|---|---|---|---|---|
| TV-001 ✔ | SYS-1 | fresh instance, network on | start server | serves within seconds; logs DB path + "Migrations completed"; catalog auto-loads (~14k objects, `/api/catalog/status.count > 10000` eventually); cron jobs registered | RUN-04 | P0 |
| TV-002 ✔ | SYS-1 | existing populated DB | restart | no re-migration side effects; "Catalog already loaded: N"; "Nothing to backfill" when storage complete | RUN-03 | P0 |
| TV-003 ✔ | SYS-3 | running | `GET /api/health` | 200; body has `status:"healthy"`, `database:"healthy"`, `worker{enabled,running,pid,restartAttempts}`, `version`, `uptime` | API-01 | P0 |
| TV-004 | SYS-3 | DB file made unreadable | `GET /api/health` | 503, `status:"unhealthy"` | SRC-04 | P1 |
| TV-005 ✔ | SYS-2 | running | `GET /api/nope` | 404 `{"message":"Not found"}`; `GET /anything` (prod build) → SPA HTML | API-08, SRC-01 | P1 |
| TV-006 | SYS-1 | fresh instance, network blocked | start | server still serves; catalog stays empty (`count:0`); error logged, no crash | SRC-01 | P1 |

## Configuration

| ID | Spec | Setup | Action | Expected | Prov | Pri |
|---|---|---|---|---|---|---|
| TV-010 ✔ | CFG-1 | fresh | `GET /api/admin/settings` | exact default document of SPEC-CFG-1 | API-03 | P0 |
| TV-011 | CFG-1/2 | set real API keys | GET settings | keys masked `••••XXXX`; POSTing the masked value back then GET again → same mask (stored key not clobbered); a connection test using the masked value still authenticates | SRC-04 | P0 |
| TV-012 ✔ | CFG-2 | — | POST settings `{"immich":{"host":"ftp://x"}}` | 400 `Only HTTP and HTTPS protocols are allowed for Immich host` | API-10 | P0 |
| TV-013 | CFG-2 | — | POST only `{"app":{"debugMode":true}}` | 200; other sections unchanged on subsequent GET | SRC-21 | P1 |
| TV-014 ✔ | CFG-3 | — | POST test-astrometry `{}` / test-immich `{}` | 400 `API key is required` / `Host and API key are required` | API-10 | P1 |

## Ingestion & storage

| ID | Spec | Setup | Action | Expected | Prov | Pri |
|---|---|---|---|---|---|---|
| TV-020 ✔ | IMG-4/6 | fresh | upload a 3000×2000 JPEG | 201 `{imageId, filename, sourceType:"local", sourceId(16 hex)}`; storage contains `processed/{id%1000 padded}/{id}/{id}_original.jpg`, `{id}_preview.jpg` (≤1920px), `{id}_thumb.jpg` (≤250px); image record has `originalPath` set, `objectType:"Deep Sky"`, `tags:[]`, `frameCount:1` | API-11, FS-01 | P0 |
| TV-021 ✔ | IMG-4 | after TV-020 | re-upload identical bytes (any filename) | 201 with the **same** `imageId`/`sourceId`; no duplicate record | API-11 | P0 |
| TV-022 ✔ | IMG-4 | — | upload with no `file` part | 400 `No file provided`; >500 MB → 413 | API-10 | P1 |
| TV-023 ✔ | IMG-5 | — | ingest-url `{"url":"ftp://x"}` / `{}` / `{"url":"::"}` | 400 with respective messages | API-10 | P1 |
| TV-024 | IMG-5 | http server with test image | ingest same URL twice | second call returns same imageId without re-downloading (verify server access log) | SRC-30 | P1 |
| TV-025 | IMG-4 | make storage dir read-only | upload | error; **no image record persisted** (list unchanged) — rollback contract | SRC-29 | P1 |

## Byte serving

| ID | Spec | Setup | Action | Expected | Prov | Pri |
|---|---|---|---|---|---|---|
| TV-030 ✔ | IMG-3 | image 1 ingested | GET thumbnail | 200 `image/jpeg`; `Cache-Control: public, max-age=31536000, immutable`; weak ETag `W/"1-thumbnail-…"`; `Accept-Ranges: none` | API-12 | P0 |
| TV-031 ✔ | IMG-3 | — | GET thumbnail with matching `If-None-Match` | 304, empty body | API-12 | P0 |
| TV-032 ✔ | IMG-3 | — | GET original with `Range: bytes=0-99` | 206; `Content-Length: 100`; `Content-Range: bytes 0-99/{size}` | API-12 | P0 |
| TV-033 | IMG-3 | — | GET original `Range: bytes=999999999-` | 416 with `Content-Range: bytes */{size}` | SRC-03 | P2 |
| TV-034 ✔ | IMG-3 | record exists, files deleted (or legacy row) | GET thumbnail | 404 `{"message":"Image file not found","backfillPending":true}` + `X-Sidereal-Backfill: pending` | API-12 | P0 |
| TV-035 | IMG-3 | — | GET `?download=1` | `Content-Disposition: attachment; filename*=UTF-8''…` | SRC-03 | P2 |
| TV-036 ✔ | IMG-3 | — | GET `/api/images/abc/thumbnail` | 400 `Invalid image id`; `GET /api/images/abc` → 404 | API-08/12 | P1 |

## Image metadata, equipment, acquisitions

| ID | Spec | Setup | Action | Expected | Prov | Pri |
|---|---|---|---|---|---|---|
| TV-040 ✔ | EQP-1 | — | POST equipment `{name:"Test Scope",type:"telescope",specifications:{focalLength:600,focalRatio:"f/6"}}` | 200 full record; POST `{type:"camera"}` → 400 | API-13 | P0 |
| TV-041 ✔ | EQP-2 | image 1 + equipment 1 | POST `/images/1/equipment {equipmentId:1,notes:"main"}`; then GET | link created; GET returns equipment merged with `{settings,notes}` | API-13 | P0 |
| TV-042 ✔ | EQP-4 | after TV-041 | add acquisitions 10×300s "Ha" and 5×120s "OIII"; GET image | image shows `frameCount:15`, `totalIntegration:1`, `filters:"Ha, OIII"`, `exposureTime:"120s - 300s"`, `focalLength:600`, `aperture:"f/6"` | API-13 | P0 |
| TV-043 | EQP-4 | after TV-042 | delete one acquisition | summary recomputed to remaining session; delete unknown id → 404 | SRC-03/21 | P1 |
| TV-044 ✔ | IMG-2 | — | PATCH `/images/1 {"targetName":"NGC 224"}` | 200; field persisted; `updatedAt` bumped | API-13 | P0 |
| TV-045 | EQP-3 | group with members {A,B}; image already linked to A | apply group to image | `{added:[B], message:"1 equipment item(s) assigned"}`; re-apply → 0 assigned | SRC-09/21 | P1 |
| TV-046 | EQP-1 | equipment linked to images/groups | DELETE equipment | equipment gone; image links and group memberships gone; acquisition filter refs nulled | SRC-21, SCHEMA-01 | P1 |

## Catalog & targets

| ID | Spec | Setup | Action | Expected | Prov | Pri |
|---|---|---|---|---|---|---|
| TV-050 ✔ | CAT-2 | loaded catalog | `GET /api/catalog/browse?limit=1&messierOnly=true` | `{items:[1 object with non-null messier], total:110, page:1, pageSize:1}` | API-09 | P0 |
| TV-051 ✔ | CAT-2/3 | — | `search?q=m31` | results include `NGC 224` (`messier:"M 31"`, `commonNames:"Andromeda Galaxy"`) — normalized-alias matching | API-09 | P0 |
| TV-052 ✔ | CAT-3 | — | `GET /api/catalog/NGC%20224` / unknown name | 200 object / 404 `Catalog object not found` | API-09 | P0 |
| TV-053 | CAT-2 | — | `browse?limit=500` | `pageSize` clamped to 100 | SRC-20 | P2 |
| TV-054 | CAT-2 | — | `browse?latitude=45&hideBelow=true` | no items with `decDeg <= -45`; southern latitude mirrors | SRC-20 | P1 |
| TV-055 ✔ | CAT-1 | fresh | wait for auto-load; `GET /api/catalog/status` | `{count:~14033, lastUpdated:iso, commitSha:40-hex}` | RUN-04, API-06 | P0 |
| TV-056 ✔ | TGT-1 | image with `targetName:"NGC 224"` | `GET /api/targets` | one summary enriched with catalog `objectType/constellation/vMag/commonNames`, `imageIds:[1]`, `thumbnailImageId:1` | API-13 | P0 |
| TV-057 ✔ | TGT-2 | — | PUT `/user-targets/NGC%20224 {notes,tags}`; PUT again with only notes | upsert; second call preserves tags; DELETE succeeds; GET after → 404 | API-13, SRC-21 | P1 |
| TV-058 | CAT-4 | image w/ tags `["M 42","astrophotography"]`, no target | POST backfill-targets | image gains a `targetName` (Messier-priority match); response counts `matched:1` | SRC-07/20 | P1 |

## Plate solving

| ID | Spec | Setup | Action | Expected | Prov | Pri |
|---|---|---|---|---|---|---|
| TV-060 ✔ | SOLVE-1 | solving disabled | POST plate-solve / bulk | 400 with configuration message | API-10 | P0 |
| TV-061 ✔ | SOLVE-1 | — | POST bulk `{"imageIds":[]}` | 400 `imageIds array is required` | API-10 | P1 |
| TV-062 | SOLVE-1/2 | valid key, solvable image (external) | POST single-image solve | job created `processing` with WS event; on success: 200 with calibration/annotations/machineTags; image gains plateSolved/ra/dec/constellation/merged tags/targetName; job `success` with result; sidecar file exists when enabled | SRC-18 | P0 |
| TV-063 | SOLVE-3 | job whose remote id vanished | POST update/{jobId} | job → `failed`; result contains expiry message + `submissionUrl`; WS failed event | SRC-18 | P1 |
| TV-064 | SOLVE-5 | `enabled+autoEnabled+key`, 5 unsolved images, maxConcurrent 3 | run worker one cycle | exactly 3 submissions; failed jobs resubmitted only when `autoResubmit` | SRC-31 | P1 |
| TV-065 | SOLVE-1 | bulk incl. a solved image | POST bulk | that item `{success:false, error:"Image already plate solved"}`, others submitted | SRC-06 | P1 |
| TV-066 | SOLVE-4 | solved image, sidecar enabled + organizeByDate, captureDate 2024-03-15 | GET `/images/{id}/sidecar` | XML containing `astro:plateSolved true`, calibration values, tags; file at `{out}/2024-03/{filename}.xmp`; `?download=true` → attachment | SRC-23 | P1 |

## Immich sync & backfill (require an Immich test double)

| ID | Spec | Setup | Action | Expected | Prov | Pri |
|---|---|---|---|---|---|---|
| TV-070 ✔ | SYNC-1 | unconfigured | POST sync-immich | 500 `Failed to sync with Immich` | API-10 | P1 |
| TV-071 | SYNC-1 | double with 3 image assets | sync | `{syncedCount:3, removedCount:0}`; records carry EXIF-derived fields, `tags:["astrophotography"]`; renditions on disk; re-sync → `{0,0}` | SRC-28 | P0 |
| TV-072 | SYNC-1 | remove one asset upstream | sync | `{syncedCount:0, removedCount:1}`; image + files + child rows gone | SRC-28/21 | P0 |
| TV-073 | SYNC-1 | `syncByAlbum:true`, no albums selected | sync | error "Sync by album is enabled, but no albums are selected." | SRC-28 | P1 |
| TV-074 | SYNC-1 | one asset >500 MB | sync | asset skipped with warning; others synced; no partial record | SRC-28 | P1 |
| TV-075 ✔ | SYNC-2 | nothing pending | POST backfill twice quickly | first 200 "Backfill started"; concurrent second → 409 (when still running) | API-10, SRC-32 | P1 |
| TV-076 | SYNC-2 | Immich image rows without local files | POST backfill; watch WS | `backfill-progress` frames then `backfill-complete {processed:N}`; `originalPath` updated; serving 404→200 | SRC-32 | P0 |

## Notifications, stats, WS

| ID | Spec | Setup | Action | Expected | Prov | Pri |
|---|---|---|---|---|---|---|
| TV-080 | NTF-1 | force a scheduled-sync failure | — | error notification "Immich Sync Failed" appears in GET /notifications; WS `source-sync-complete {success:false}` | SRC-19 | P1 |
| TV-081 | NTF-1 | ≥2 notifications | POST acknowledge-all | GET returns `[]`; WS `notifications-updated`; admin card disappears (UI) | SRC-04, TEST-09 | P1 |
| TV-082 ✔ | STAT-1 | fresh | GET /stats | zeros document with exact field names (`plateSolved`, `totalHours`, `uniqueTargets`, `objectTypeCounts`, `plateSolvingStats{…}`) | API-02 | P0 |
| TV-083 | STAT-1 | images with tags incl. star names from solving | GET /tags | star-name tags (`The star …`) excluded; catalog designations and `astrophotography` counted; ≤20 entries desc | SRC-04/26 | P1 |
| TV-084 ✔ | WS-1 | running | connect `ws://host/ws` | connection accepted; frames are JSON `{event,data}` | WS-01 | P0 |
| TV-085 | WS-1 | two browser clients | trigger acknowledge | both receive `notifications-updated` (broadcast, not unicast) | SRC-25 | P2 |

## Admin/database/docker

| ID | Spec | Setup | Action | Expected | Prov | Pri |
|---|---|---|---|---|---|---|
| TV-090 ✔ | ADM-1 | SQLite | GET /admin/database | `{type:"sqlite", path, sizeBytes>0, lastModified}` | API-05 | P1 |
| TV-091 | ADM-1 | SQLite | GET /admin/database/backup | `application/x-sqlite3` attachment `sidereal-backup-*.db`; restoring it into a new instance reproduces records | SRC-04 | P1 |
| TV-092 | ADM-1 | PostgreSQL mode | GET backup | 400 with pg_dump message | SRC-04 | P2 |
| TV-093 | DCK-1 | container, default env | start | healthcheck passes ≤40 s; two processes (server+worker); killing worker PID → auto-restart ≤30 s; SIGTERM → clean exit | DOCKER-02 | P1 |
| TV-094 | DCK-1 | container `PUID=99 PGID=100` | start; write files | data/sidecars owned by 99:100 | DOCKER-02 | P2 |
| TV-095 | JOB-1 | create dir `processed/000/999999` w/o DB row | run orphan sweep (03:00 or invoke) | directory removed; log line | SRC-19/22 | P2 |

## UI workflows (Playwright-style, per existing suite)

| ID | Spec | Action | Expected | Prov | Pri |
|---|---|---|---|---|---|
| TV-100 | UI 7.1 | open `/` | title "Sidereal"; header, nav, Sync Immich, search box, sidebar cards; gallery grid or empty state | TEST-01 | P0 |
| TV-101 | UI 7.1 | visit `/?image=1` (image exists) | overlay open (Close button, Plate Solution/Technical Details/Tags sections); closing strips the param | TEST-01 | P0 |
| TV-102 | UI 7.1 | inspect gallery `img` tags | all `src` match `/api/images/{id}/(thumbnail|preview)`; zero `/api/assets/` references; responses cacheable (max-age=31536000) | TEST-03 | P0 |
| TV-103 | UI 7.9 | block `/api/**`, load `/`, `/equipment`, `/admin`, `/plate-solving` | page shells render with headings; no blank screen | TEST-04 | P1 |
| TV-104 | UI 7.3 | open equipment add form | submit disabled until name+type; six type options; spec add disabled until key+value | TEST-06 | P1 |
| TV-105 | UI 7.7 | admin page | three config sections; Immich URL input `type=url`, API key `type=password`; astrometry fields hidden until enabled; Save Settings button | TEST-02 | P0 |
| TV-106 | UI 7.5 | `/sky-map` with 0 solved images | empty state + link to plate solving; with markers: count subtitle; "View in Gallery" → `/?image={id}` | TEST-08 | P1 |
| TV-107 | UI 7.7 | >5 notifications | card shows 5, "Show all" expands, "Show less" collapses; "Acknowledge All" hidden at exactly 1 | TEST-09 | P1 |
| TV-108 | UI 7.8 | visit `/nonexistent` | "404 Page Not Found" card | TEST-04 | P1 |
