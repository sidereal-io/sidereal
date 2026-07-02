# 7. UI Workflow Specifications

UI evidence comes from client source (SRC-33..37) and Playwright assertions (TEST-01..09); no live browser session was run. Shared chrome first, then per-route workflows.

## 7.0 Shared chrome (all routes)

- Header with logo (`alt="Sidereal Logo"`), `h1` "Sidereal", nav links (Gallery `/`, Targets, Equipment, Plate Solving, Sky Map, Locations, Admin gear), a **Sync Immich** button, a notification bell with unacknowledged count, and a GitHub link (`https://github.com/mstelz/sidereal`, opens in new tab). Active nav link styled distinctly (`text-foreground` vs `text-muted-foreground`). Header renders even when all API calls fail. Document title: "Sidereal". (TEST-01, TEST-04, TEST-05)
- **Sync Immich** button: POST `/api/immich/sync-immich`; on completion invalidates gallery + stats queries. (SRC-37)
- Data layer behavior: queries never auto-refetch or retry; errors surface as thrown messages `"{status}: {body}"`; real-time WS events drive refetches instead. (SRC-35, SRC-34)
- Dark theme applied globally. (SRC-33)

## 7.1 Gallery home — `/`

- **Purpose**: browse, filter, and inspect the image collection.
- **Data**: `GET /api/images` (server-side filters: objectType, tags, plateSolved, constellation, equipmentId), `GET /api/stats`, `GET /api/equipment`, `GET /api/tags`.
- **User actions & behavior**:
  - Free-text search box (`placeholder "Search astrophotography images..."`) filters client-side by title substring; retains/clears input. Advanced filter button exposes date-range and min-integration filters (client-side).
  - Filter bar + sidebar Popular Tags (clicking a tag toggles it in the tag filter).
  - Grid shows 12 images initially; **Load More** button (text includes "load more images") reveals +12 at a time; changing filters resets to 12; button hidden when everything is visible.
  - Each card: image (`src` = `/api/images/{id}/(thumbnail|preview)` — never the Immich proxy), badge "Plate Solved" or "No Plate Data".
  - Clicking a card opens a full-screen **image overlay** and rewrites the URL to `/?image={id}` (bookmarkable). Deep-linking `/?image={id}` opens the overlay once images load; closing (button `aria-label="Close"`) removes the param.
  - Overlay content: viewer with expand control, "Plate Solution" section, "Technical Details" section, "Tags" section; equipment-based filtering can be triggered from the overlay (sets `equipmentId` filter and closes).
- **Loading**: gallery renders a loading state while images load (skeletons); **empty state** shown when no images. Sidebar cards: Astrometry.net Status, Recent Activity, Popular Tags, and a "Submit New Image" link → `/plate-solving`.
- **Real-time**: `plate-solving-update` and `source-sync-complete` events refetch images + stats.
- **Provenance**: SRC-36, TEST-01, TEST-03. **Confidence**: High.

## 7.2 Targets — `/targets`

- Browse imaged targets (grouped summaries with catalog enrichment); target detail dialog shows the image list; integrates the catalog browser/user-target annotations (search, filters, DSS thumbnails via `/api/catalog/thumbnail/{name}`, visibility hints computed client-side from latitude).
- **API deps**: `/api/targets`, `/api/catalog/browse|search|status`, `/api/user-targets/*`, `/api/catalog/thumbnail/*`.
- **Provenance**: SRC-33, SRC-37, SRC-40. **Confidence**: Medium (page internals sampled, not fully traced).

## 7.3 Equipment — `/equipment`

- Heading "Equipment Catalog" + Add button. List of equipment cards with edit/delete buttons, or an empty state.
- **Add/edit form**: name + type required — submit disabled until both filled; type dropdown offers telescope/camera/mount/filter/accessory/software; per-type known specification fields plus free key/value spec entry ("Add spec" disabled until both key and value present); cancel closes without saving.
- **Groups**: create/delete equipment groups with member selection (list shows members).
- **API deps**: `/api/equipment*`, `/api/equipment-groups*`. Errors from API (page shell still renders when API blocked).
- **Provenance**: TEST-06, SRC-37, SCHEMA-04. **Confidence**: High for asserted behaviors.

## 7.4 Plate solving — `/plate-solving`

- Heading + description; **five stats cards** with numeric values (totals/pending/successful/failed/unsolved family from `/api/stats` + jobs).
- Image selection card: search input filters candidates; "show only unsolved" checkbox; select-all/deselect-all toggle; count text; empty state when no images.
- Submitting selected images → `POST /api/plate-solving/bulk`; success invalidates images/jobs/stats queries; per-image results surfaced via toasts; job list reflects `processing → success|failed` transitions live via `plate-solving-update`.
- **API deps**: `/api/images`, `/api/plate-solving/jobs`, `/api/plate-solving/bulk`, WS.
- **Provenance**: TEST-07, SRC-37, SRC-34. **Confidence**: High for asserted; Medium for toast details.

## 7.5 Sky map — `/sky-map`

- All-sky viewer (Aladin container) plotting one marker per plate-solved image (`GET /api/sky-map/markers`).
- Subtitle displays marker count when markers exist; **empty state** with a link to plate solving when none.
- Marker popup → "View in Gallery" navigates to `/?image={id}` (opens the gallery overlay).
- **Provenance**: TEST-08, SRC-37, SRC-13. **Confidence**: High.

## 7.6 Locations — `/locations`

- Manage saved observing sites: list, create/edit via a modal with a map picker (Leaflet), delete. Fields: name, latitude, longitude, altitude, description.
- **API deps**: `/api/locations*`.
- **Provenance**: SRC-33, SRC-10, package deps. **Confidence**: Medium.

## 7.7 Admin — `/admin`

- Heading "Admin Settings" (renders even with API blocked) + one **Save Settings** button persisting the whole document via `POST /api/admin/settings`.
- **Sections** (three primary asserted: Immich, Astrometry, Application; plus catalog, database, sidecar, notifications):
  - **Immich**: Server URL input (`type=url`), API Key (`type=password`, shows masked value), Auto Sync switch, Sync-by-album switch (album multi-select loaded via `POST /api/immich/albums`), Test Connection button (uses `test-immich-connection`; masked key honored).
  - **Astrometry**: master enable switch; when enabled, reveals API key (`password`) and related fields; when disabled, those fields are hidden. Test Connection via `/api/test-astrometry-connection`.
  - **Application**: Debug Mode switch (toggle + persist round-trip asserted).
  - **Catalog**: shows `{count, lastUpdated, commitSha}` from `/api/catalog/status`; actions to load/reload and check updates.
  - **Database**: shows type/path/size from `/api/admin/database`; backup download link.
  - **Notifications card**: appears only when unacknowledged notifications exist; lists first 5 with type icons and messages; "Show all"/"Show less" toggling beyond 5; per-item acknowledge buttons; "Acknowledge All" only when more than one; card disappears when all acknowledged.
- **Validation feedback**: invalid Immich host rejected server-side with 400 message; connection tests report success/failure messages inline/toast.
- **Provenance**: TEST-02, TEST-09, SRC-37, API-03/API-10. **Confidence**: High for asserted behaviors.

## 7.8 Not found — any unknown route

- Card with alert icon: "404 Page Not Found" and hint text ("…forget to add the page…"). Client-side only (server always returns the SPA shell for non-API paths).
- **Provenance**: TEST-04, SRC-01. **Confidence**: High.

## 7.9 Degraded-API behavior (all pages)

With every `/api/**` request failing, the app still renders: header + title, search filters on home, Equipment heading, Admin heading, Plate Solving heading. No white-screen. Data areas show empty/error states rather than crashing. **Provenance**: TEST-04. **Confidence**: High.
