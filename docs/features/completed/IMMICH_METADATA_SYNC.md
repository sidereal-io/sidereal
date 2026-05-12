# Immich Metadata Writeback

## Problem
Only latitude and longitude are synced back to Immich. Users want all available metadata synced, with the ability to enable/disable this in the admin panel.

## Current State
- `updateImmichAssetMetadata()` in `images.ts` only sends `latitude` and `longitude` via Immich's `PUT /api/assets`
- Called only when image coordinates are updated
- Admin settings panel has `autoSync` field but no toggle for metadata writeback
- No sync for: description, tags, constellation, equipment info, acquisition data, plate solving results

## Immich API Capabilities
- `PUT /api/assets`: supports `description`, `latitude`, `longitude`, `rating`, `dateTimeOriginal`
- Tags: managed via separate Immich tag API endpoints
- Bulk updates: single call can update multiple asset fields

## Solution

### Metadata to Sync
Build a rich description string from all available Sidereal data:
- Object name/title
- Constellation
- Equipment list (telescope, camera, mount, filters)
- Acquisition summary (per-filter breakdown, total integration)
- Plate solving results (RA/Dec, pixel scale, field of view)
- Coordinates (latitude/longitude) — already implemented
- Tags — sync Sidereal tags to Immich tags

### Admin Settings (`apps/client/src/pages/admin.tsx`)
Add a "Metadata Sync" section with:
- Master toggle: Enable/disable metadata writeback to Immich
- Granular checkboxes: description, coordinates, tags (allow choosing what to sync)
- "Sync All Images" bulk action button

### Backend
- **New service: `apps/server/src/services/immich-sync.ts`**: Centralized sync logic
  - `syncImageMetadata(imageId)`: Builds and sends all enabled metadata to Immich
  - `syncAllImages()`: Bulk sync for all images
  - Reads admin settings to determine what to sync
- **Trigger points**: After image update, plate solve completion, acquisition data change, equipment change
- **Per-image button**: "Sync to Immich" in image overlay

### Config Storage
- Store sync settings in the existing config service (immich settings object)
- Fields: `metadataSyncEnabled`, `syncDescription`, `syncCoordinates`, `syncTags`

## Files
- `apps/server/src/services/immich-sync.ts` (new)
- `apps/server/src/routes/images.ts` (trigger sync on updates)
- `apps/server/src/services/storage.ts` (config helpers)
- `apps/client/src/pages/admin.tsx` (settings UI)
- `apps/client/src/components/image-overlay.tsx` (per-image sync button)
