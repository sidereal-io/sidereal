# 4. Domain Model

Externally meaningful Sidereal concepts, their relationships, lifecycle states, invariants, and user-visible constraints. This describes what the data *means to users and API clients* — not table layout (see §9 only where byte-compatibility requires it).

## 4.1 Image (photo/asset)

The central entity: one astrophotograph tracked by Sidereal.

**Identity & provenance**
- Numeric `id` (assigned by Sidereal, monotonically increasing; embedded in URLs and on-disk layout).
- `(sourceType, sourceId)` identifies where the image came from and deduplicates ingestion:
  - `immich` — `sourceId` is the Immich asset UUID.
  - `local` — `sourceId` is the first 16 hex chars of the SHA-256 of the file **content** (same bytes ⇒ same image, regardless of filename). (API-11)
  - `url` — `sourceId` is the first 16 hex chars of the SHA-256 of the **URL string** (same URL ⇒ same image, even if remote content changed). (SRC-30)

**User-visible fields** (all nullable unless noted): `title`*, `filename`*, `originalPath`, `captureDate`, capture settings (`focalLength`, `aperture`, `iso`, `exposureTime`, `frameCount`, `totalIntegration` hours), legacy free-text equipment (`telescope`, `camera`, `mount`, `filters`), location (`latitude`, `longitude`, `altitude`), plate-solve results (`plateSolved` bool, `ra`, `dec` as strings, `pixelScale`, `fieldOfView` string like `"84.4'"`, `rotation`, `astrometryJobId`), classification (`tags` string array, `objectType`, `constellation`, `targetName`), `description`, `createdAt`/`updatedAt`. (SCHEMA-01, API-11, API-13)

**Ingestion defaults**: Immich-synced images get `tags:["astrophotography"]`, `objectType:"Deep Sky"`, `frameCount:1`, EXIF-derived capture settings/coordinates, `aperture` formatted `f/{n}`, `camera` = "{make} {model}", `telescope` = lens model. Local/URL uploads get empty tags, `objectType:"Deep Sky"`, `frameCount:1`. (SRC-28/29/30, API-11)

**Lifecycle**
1. *Ingested*: record created, then original + preview + thumbnail written to local storage; `originalPath` set. If file processing fails, the record is rolled back (all-or-nothing). (SRC-29/30/28)
2. *Backfill-pending* (legacy Immich images only): record exists but files absent → byte-serving returns 404 with `backfillPending: true`; backfill job repairs. (SRC-32, API-12)
3. *Plate-solved*: a successful solve sets `plateSolved`, coordinates, pixel scale, field of view, rotation, constellation (computed from RA/Dec), merges filtered machine tags into `tags`, auto-assigns `targetName` from catalog match, and (if enabled) writes an XMP sidecar. (SRC-18)
4. *Removed*: only via Immich sync reconciliation (asset gone from Immich ⇒ image deleted along with jobs, links, acquisitions, files). No user-facing delete for local/url images. (SRC-28, SRC-21)

**Invariants**
- An image's three renditions live together; deleting the image removes them.
- Summary fields (`frameCount`, `totalIntegration`, `filters`, `exposureTime`, `focalLength`, `aperture`) are **derived** whenever acquisitions or equipment links change (see 4.5) — manual PATCH edits to them can be overwritten by the next recompute. (SRC-21, SRC-38, API-13)

## 4.2 Image source

A pluggable origin for images. Three exist: **Local Upload** (`local`), **Web / URL Upload** (`url`), **Immich** (`immich`). Each reports `{sourceType, lastSync (always null today), imageCount, healthy, message?}` plus a display name, and supports a connection test (local/url trivially OK; Immich checks host+key against the albums endpoint). Only Immich supports sync. (SRC-27..30, API-07)

## 4.3 Album/gallery grouping

Sidereal has **no album entity of its own**. Immich albums appear only as a *sync scope selector* (`syncByAlbum` + `selectedAlbumIds`). Gallery organization is by filters (tags, object type, constellation, plate-solved, equipment) and by **targets**. (SRC-05, SRC-28)

## 4.4 Target and catalog object

- **Catalog object**: one deep-sky object from the OpenNGC catalog (NGC/IC plus Messier addendum; ~14,033 objects observed). Fields: canonical `name` (normalized, e.g. `NGC 224`, `IC 434`, `M 45`), `type` code (e.g. `G`, `*Ass`), sexagesimal `ra`/`dec` plus decimal `raDeg`/`decDeg`, `constellation` (3-letter), sizes (`majorAxis`/`minorAxis` arcmin), magnitudes (`bMag`, `vMag`), `surfaceBrightness`, `hubbleType`, cross-refs (`messier` like `M 31`, `ngcRef`, `icRef`), `commonNames`, `identifiers`. Names are unique. The catalog is replaceable wholesale from upstream; a commit SHA + timestamp + count are tracked. (SRC-20, SRC-39, API-06, API-09)
- **Target** (imaged target): a *virtual* grouping of the user's images by `targetName` (falling back to `title` when unset). A target summary exposes `targetName, imageCount, totalIntegrationHours, thumbnailImageId (most recent image), objectType, constellation, vMag, commonNames, latestCaptureDate, imageIds`, enriched from the catalog when the name (or Messier designation) matches. Sorted by image count desc, then latest capture. (SRC-38, API-13)
- **Target auto-matching**: image tags → normalized names → exact catalog match, plus Messier cross-reference; matches prioritized Messier-first then brightest. First match becomes `targetName`. Runs after each successful solve and via the backfill-targets action. (SRC-20, SRC-18, SRC-07)
- **User target**: personal annotation on a catalog object — `catalogName` (unique), free `notes`, `tags` array. Upsert semantics; independent of whether the object was ever imaged. (SRC-12, API-13)

## 4.5 Equipment, equipment group, acquisition

- **Equipment**: `name`* and `type`* (`telescope|camera|mount|filter|accessory|software` — the UI offers these six; the API accepts any string), free-form `specifications` object (known keys per type documented in the UI, e.g. telescope `focalLength`/`apertureDiameter`/`focalRatio`/`design`), `description`, `cost`, `acquisitionDate`, optional `imageUrl`. (SCHEMA-04, SRC-08, TEST-06)
- **Image–equipment link**: which equipment produced an image, with per-image `settings` object and `notes`. Adding/removing links triggers image summary recompute: the linked telescope's `focalLength`/`focalRatio` specs flow into the image's `focalLength`/`aperture`. (SRC-21, SRC-38, API-13)
- **Equipment group**: named reusable rig preset (`name`*, `description`, member equipment list). "Apply to image" adds only members not already linked and reports what was added. Deleting a group removes only the grouping. (SRC-09, SRC-21)
- **Acquisition entry**: one filter/exposure session row for an image — `frameCount`* , `exposureTime`* (seconds), optional `filterId` (equipment ref, null-on-delete) or free `filterName`, `gain`, `offset`, `binning`, `sensorTemp`, `date`, `notes`. Aggregation rule (observed): image `frameCount` = Σ frames; `totalIntegration` = Σ(frames×exposure)/3600 rounded to 3 decimals; `filters` = comma-joined distinct filter names; `exposureTime` = `"300s"` when uniform else `"120s - 300s"` (min–max). (SRC-38, API-13)

## 4.6 Plate solve (job)

A solve attempt for an image against Astrometry.net.

- Fields: `imageId`, `astrometrySubmissionId`, `astrometryJobId`, `status`, `submittedAt`, `completedAt`, `result` (JSON: calibration fields `ra, dec, pixscale, radius, orientation, parity, width_arcsec, height_arcsec` + `annotations` array; on failure: `{error, submissionId, astrometryJobId, submissionUrl, jobUrl}`).
- Status lifecycle: `processing` (created at submission; schema default `pending` exists but jobs are created as `processing`) → `success` | `failed`. Failure reasons surfaced to users: upstream could-not-solve, expired job (~30 days), not found. (SRC-18, SCHEMA-01)
- Multiple jobs may exist per image; "the" solve is the most recent successful one. Success sets image fields (see 4.1) — image and job updates are the same user-visible transaction outcome, plus a `plate-solving-update` WS event.
- What gets uploaded to Astrometry.net is the locally stored **preview** rendition (≤1920px JPEG), not the original. (SRC-18)
- Concurrency: automatic solving submits at most `maxConcurrent` (default 3) in-flight jobs; failed jobs are only re-submitted automatically when `autoResubmit` is on. (SRC-31)

## 4.7 Admin setting

A persisted key→JSON-value store, presented to users as one settings document with sections `immich`, `astrometry`, `sidecar`, `app` (defaults in SPEC-CFG-1) plus internal catalog metadata keys (`catalog_lastUpdated`, `catalog_commitSha`, `catalog_objectCount`). API keys are write-only in practice: reads return masked values (`••••` + last 4), and masked values sent back are ignored in favor of stored ones. Updates are per-top-level-key upserts; a document POST replaces the sections it includes. (SRC-17, SRC-21, SRC-04, API-03)

## 4.8 Notification

Operational alert for the user: `type` (`error|warning|info|success`), `title`, `message`, optional `details` object, `acknowledged` flag, `createdAt`. Created by background failures (e.g. "Immich Sync Failed", "Cron Job Error"). Only unacknowledged ones are listed; acknowledged ones are purged after 30 days by the nightly cleanup. Acknowledge (one or all) broadcasts `notifications-updated`. UI shows a bell count and an admin-page card (first 5, expandable). (SCHEMA-04, SRC-19, SRC-04, TEST-09)

## 4.9 Processed image

The local rendition set for an image: original (as ingested, ≤500 MB), preview (≤1920px JPEG q85), thumbnail (≤250px JPEG q80), EXIF orientation applied. Addressed only through `/api/images/{id}/{size}`; treated as immutable (1-year immutable cache headers, mtime-based ETag). (SRC-22, API-12, FS-01)

## 4.10 Worker job / background activity

Users observe background work through: cron job list (Immich sync, notification cleanup, orphan sweep), the health endpoint's `worker` block, WS progress events, notifications on failure, and log output. The plate-solving worker is a separate process in production; the orphan sweep guarantees eventual consistency between DB records and on-disk image directories. (SRC-19, SRC-24, SRC-31, SRC-32, API-01)

## 4.11 Location

Saved observing site: `name`*, `latitude`*, `longitude`*, optional `altitude`, `description`. Full CRUD; no observed server-side coupling to images (images carry their own coordinates). (SRC-10, API-13; see Conflict C7 for the docs' claim about Immich write-back.)

## 4.12 Relationships (summary)

```
ImageSource (local|url|immich) --ingests--> Image
Image 1--* PlateSolvingJob
Image 1--* AcquisitionEntry (optional filter -> Equipment)
Image *--* Equipment (via link w/ settings+notes)
EquipmentGroup *--* Equipment ; Group --apply--> Image links
Image.targetName --(name/Messier match)--> CatalogObject (soft reference)
UserTarget --catalogName--> CatalogObject (soft reference, unique per name)
Notification, AdminSetting, Location: standalone
```
Soft references: `targetName`/`catalogName` are strings, not enforced foreign keys — a target can name a nonexistent catalog entry and grouping still works (falls back to image-derived metadata). (SRC-38)
