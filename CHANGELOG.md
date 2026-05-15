# Changelog

All notable changes to Sidereal will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.10.0] - 2026-05-15

### Changed
- **Rebrand**: Renamed project from Skymmich to Sidereal across all code, configuration, documentation, and Docker artifacts. See the [Migration Guide](docs/MIGRATION.md) for upgrade steps.
- **Dependencies**: Updated production dependencies — @tanstack/react-query 5.100.10, framer-motion 12.38.0, hono 4.12.18, lucide-react 1.14.0, react 19.2.6, react-dom 19.2.6, react-resizable-panels 4.11.0, tailwind-merge 3.6.0, zod 4.4.3, and others.
- **Dev Dependencies**: Updated development dependencies including drizzle-kit, esbuild, postcss, tailwindcss, tsx, vite, and various @types packages.
- **CI**: Replaced QEMU ARM64 emulation with native `ubuntu-24.04-arm` runners for faster multi-arch builds.
- **CI**: Use tag references instead of digests for Trivy and SBOM steps.

### Security
- **npm**: Bumped to 11.14.1 to resolve CVE-2026-42338.

### Breaking Changes
- **Environment variable**: `SKYMMICH_PORT` → `SIDEREAL_PORT`
- **Docker volumes**: `skymmich-*` → `sidereal-*` (existing volumes must be renamed or remapped)
- **PostgreSQL defaults**: database name and user renamed from `skymmich` to `sidereal`
- **SQLite default path**: `/app/config/skymmich.db` → `/app/config/sidereal.db`
- **Container system user**: `skymmich` → `sidereal`
- **Unraid template paths**: `/mnt/user/appdata/skymmich/` → `/mnt/user/appdata/sidereal/`
- **Asset filename**: `skymmich-transparent.png` → `sidereal-transparent.png`

## [0.9.2] - 2026-04-08

### Security
- **hono**: Updated to 4.12.12 to fix cookie name validation bypass, IP matching in ipRestriction(), path traversal in toSSG(), and middleware bypass via repeated slashes in serveStatic.
- **@hono/node-server**: Updated to 1.19.13 to fix middleware bypass via repeated slashes in serveStatic.
- **vite**: Updated to 8.0.5 to fix path traversal in optimized deps, `server.fs.deny` bypass, and arbitrary file read via WebSocket.

## [0.9.1] - 2026-03-31

### Changed
- **Dependencies**: Updated production dependencies — @tanstack/react-query 5.95.2, drizzle-orm 0.45.2, hono 4.12.9, lucide-react 1.7.0, react-resizable-panels 4.8.0, recharts 3.8.1.
- **Dev Dependencies**: Updated TypeScript to 6.0.2 and Vite to 8.0.3.
- **GitHub Icon**: Replaced lucide-react `Github` icon (removed in v1.0) with inline SVG.
- **TypeScript Config**: Added `ignoreDeprecations: "6.0"` for TypeScript 6.x `baseUrl` deprecation.
- **CI**: Upgraded GitHub Actions workflows from Node 20 to Node 24.

### Security
- **picomatch**: Override to >=4.0.4 to fix method injection vulnerability (GHSA).
- **brace-expansion**: Updated to fix zero-step sequence hang vulnerability.

## [0.9.0] - 2026-03-23

### Added
- **Sky Map Equipment FOV Overlay**: Select telescope and camera from the sky map to display a real-time field-of-view rectangle overlay, calculated from focal length, pixel size, and sensor resolution. Includes a toggle to show/hide the FOV and displays which equipment specs are missing when the overlay can't be computed.
- **Sky Map UI Improvements**: Repositioned Aladin Lite controls — coordinates and zoom in bottom-left, overlays menu in bottom-right, telescope/camera selectors in top-left. Added fullscreen support with responsive control placement.
- **Database Admin Section**: New admin panel section showing database engine, file size, and last modified timestamp. Includes a one-click backup download button for SQLite databases.
- **Database Migration Script**: New `migrate-db` tool for migrating data between PostgreSQL and SQLite in either direction. Discovers tables and column types dynamically from database metadata — no manual updates needed when the schema changes. SQLite targets automatically run Drizzle migrations to ensure the schema exists.
- **Auto-Migration on Startup**: Set `AUTO_DB_MIGRATE_FROM` environment variable to automatically migrate data during Docker container startup. Supports one-time migration with a marker file, optional SQLite reset, and credential masking in logs.
- **Target Name Column**: New `target_name` column on astrophotography images (migration 0008).

### Changed
- **Default Database**: SQLite is now the default database for all deployments including Docker and UnRAID. No external database setup required — data is stored in `/app/config/skymmich.db`.
- **PostgreSQL Optional**: PostgreSQL remains fully supported as an optional external database. Use `docker-compose.postgres.yml` as a compose override or set `DATABASE_URL` to enable it.
- **Docker Compose**: Simplified to a single-container setup by default. PostgreSQL service moved to a separate `docker-compose.postgres.yml` override file.
- **UnRAID Template**: Removed PostgreSQL as a requirement. `DATABASE_URL` is now optional with an empty default.
- **Dockerfile**: `better-sqlite3` is now included in the production image for built-in SQLite support.
- **SQLite Path**: Configurable via `SQLITE_DB_PATH` env var, defaults to `/app/config/skymmich.db` in production and `local.db` in development.
- **SQLite Migrations Path**: Migrations folder is now resolved dynamically across multiple candidate paths, fixing issues when running inside Docker.
- **Build Order**: `build:docker` now runs vite, copy-assets, then esbuild in the correct order to ensure the migration script is bundled properly.
- **XMP Sidecar**: Marked as experimental in documentation — feature is under active development and may not work as intended in all configurations.

### Fixed
- **Tag Filtering**: Image tag filtering now works on both SQLite (using `json_each`) and PostgreSQL (using native array overlap). Previously only PostgreSQL was supported.

## [0.8.0] - 2026-03-22

### Changed
- **Server Framework**: Migrated from Express to Hono with `@hono/node-server` for HTTP routing. All 13 route files converted to Hono's context-based handler API.
- **HTTP Client**: Replaced axios with native `fetch` across all server services and routes. Uses `AbortSignal.timeout()` for request timeouts and native `FormData` for multipart uploads.
- **WebSocket**: Replaced Socket.IO with native WebSocket using the `ws` package on the server and browser-native `WebSocket` on the client. Added automatic reconnection with exponential backoff.
- **Asset Proxy**: Immich asset proxy now streams responses via `fetch()` passthrough instead of axios stream piping.
- **Mobile Navigation**: Added hamburger menu for mobile viewports. Navigation links, Sync Immich, and Admin Settings are accessible from a slide-out drawer on the right. Desktop header is unchanged.
- **Toast Position**: Toast notifications now appear at the bottom of the screen on mobile instead of the top.

### Removed
- **Dependencies**: Removed express, axios, form-data, socket.io, socket.io-client, cors, passport, passport-local, connect-pg-simple, memorystore, and their associated type packages.
- **Stale Overrides**: Removed `qs` and `socket.io-parser` npm overrides that were only needed for Express/Socket.IO transitive dependencies.

### Fixed
- **Missing Dependency**: Added `pg` as an explicit dependency. It was previously resolved as a transitive dependency of `connect-pg-simple`.
- **Connection Test Responses**: Added missing `success` field to Immich and Astrometry test connection responses so the frontend can correctly show green/red status styling.
- **Catalog Backfill Counter**: Fixed matched count never incrementing during catalog backfill.
- **Notification Timestamps**: Fixed field mismatch (`timestamp` → `createdAt`) in notification display.
- **Plate Solving Timeout**: Added 60-minute timeout to plate solving poll loop to prevent infinite hangs.
- **Remote Image URLs**: Fixed `RemoteImage` component stripping query parameters from image URLs.
- **Redacted Key Handling**: Properly handle redacted API keys in test connection and album endpoints to avoid overwriting stored keys.
- **Async File I/O**: Replaced blocking `readFileSync`/`writeFileSync` with async `fs` operations.
- **Astrometry HTTPS**: Changed all Astrometry.net API calls from HTTP to HTTPS.
- **API Key Masking**: Mask API keys in `GET /admin/settings` response, showing only the last 4 characters.
- **Database URL Redaction**: Redact `DATABASE_URL` password in Docker startup logs.
- **Stats Query**: Rewrote `getStats()` to use SQL aggregation instead of loading all rows into memory.
- **Dead Code Removal**: Removed unused `apiToken` localStorage code from the client.
- **Shared Notification Type**: Created shared `Notification` type, removing 3 duplicate interface definitions.
- **Immich Sync Refactor**: Extracted Immich image sync into a service layer; cron calls service directly.
- **Query Filters**: Replaced fragile positional `queryKey` array with typed `QueryFilters` object.
- **Schema Improvements**: Added missing foreign key references to SQLite schema and sync comments between pg and sqlite files.

## [0.7.2] - 2026-03-21

### Fixed
- **ReDoS Vulnerability**: Fixed polynomial regular expression in catalog name normalization that could cause denial-of-service with crafted input.
- **Thumbnail Path Safety**: Serve cached thumbnails via static middleware instead of manual file reads, delegating path safety to the framework.
- **Thumbnail Rate Limiting**: Added global throttle on external survey image fetches to prevent abuse of the upstream API.

## [0.7.1] - 2026-03-20

### Fixed
- **Docker Permissions**: Added `/app/cache` to startup permission management. This ensures that the container automatically handles ownership of survey images and thumbnails regardless of host environment or PUID/PGID settings.
- **Persistence**: Fixed missing cache volume mappings in production Docker Compose and Unraid templates.

## [0.7.0] - 2026-03-20

### Added
- **Deep Sky Catalog & Targets**: New "Targets" page for browsing and filtering astronomical objects (Messier and NGC/IC catalogs).
- **Advanced Target Filtering**: Search by name/aliases, multi-select object types, filter by constellation, magnitude range, and minimum size.
- **Survey Thumbnails**: Automatic DSS2 survey image previews from Aladin Lite (hips2fits) with local disk caching.
- **"Best Now" Sorting**: Intelligently rank targets based on current date and observer location to find what's best to image tonight.
- **Visibility Filtering**: Support for hiding targets currently below the horizon based on managed location coordinates.
- **Auto-Matching**: Automatically assign catalog targets to images after successful plate solving.
- **Target Picker Modal**: Interactive search tool to manually assign catalog targets to gallery images.
- **Backfill Administrative Tool**: New "Backfill Targets" button in Admin to re-match all existing plate-solved images against the catalog.
- **Immich Metadata Sync**: Full writeback of metadata to Immich, including image description, constellation, and celestial coordinates.
- **Metadata Configuration**: Granular admin toggles to enable/disable specific metadata fields for Immich sync.
- **Gallery Equipment Filter**: Clicking an equipment badge in the image overlay now automatically filters the gallery to show all images using that equipment.

### Changed
- **Header Navigation**: Added "Targets" to the main site navigation.
- **Dockerfile**: Added persistent cache directory for thumbnails (`/app/cache/thumbnails`).
- **App Layout**: Registered new `/targets` route and updated global UI components.
- **Persistence**: Added new `/app/cache` volume requirement. **Existing Docker and Unraid users should manually add this path mapping to ensure survey images and thumbnails persist across restarts.**

### Fixed
- **Thumbnail Cache**: Improved thumbnail serving performance via disk-based caching and immutable headers.
- **Plate Solving**: More robust target matching during the post-processing phase of plate solving jobs.

## [0.6.1] - 2026-03-19

### Fixed
- **PostgreSQL Migration**: Added missing `created_at` column to `equipment_group_members` table in PostgreSQL DDL.

### Security
- **socket.io-parser CVE**: Overrode socket.io-parser to >=4.2.6 to fix high-severity unbounded binary attachments vulnerability.
- **Docker Image**: Patched node-tar and zlib CVEs in Docker image.
- **Dockerfile Lint**: Added hadolint ignore for DL3002 since root is required for PUID/PGID remapping at startup.

### Changed
- **Dependencies**: Updated production dependencies (better-sqlite3, framer-motion, nanoid, openseadragon, react-resizable-panels).

## [0.6.0] - 2026-03-11

### Added
- **Equipment Groups**: Create named equipment groups (e.g., "Deep Sky Rig") to bundle telescopes, cameras, and accessories together for quick assignment to images.
- **Apply Group to Image**: Apply an equipment group to an image from the gallery overlay, with preview of members and duplicate detection.
- **Equipment Cost & Acquisition Date**: Track purchase cost and acquisition date for equipment items.
- **Real-time Notifications**: Notifications now use React Query with socket.io events for instant updates across tabs.

### Changed
- **Dependencies**: Updated production and development dependencies (axios, react-day-picker, react-resizable-panels, autoprefixer, @types/node).
- **GitHub Actions**: Updated docker/setup-buildx (v4), docker/login (v4), docker/metadata (v6), docker/build-push (v7), actions/upload-artifact (v7), aquasecurity/trivy-action (0.35.0).
- **Dockerfile**: Fixed linting errors (DL3003, DL3042) and improved caching efficiency.
- **Docs**: Consolidated feature documentation into `docs/features/` directory.

### Fixed
- **GHCR Prune**: Fixed image pruning workflow to preserve semver release tags.
- **Equipment Form**: Restructured form layout to row-based and improved dark mode contrast for inputs and labels.
- **Database**: Added missing `original_path` column to SQLite schema migration.

## [0.5.1] - 2026-03-01

### Fixed
- **Immich Auto-Sync**: Fixed automatic sync cron job failing with HTTP 404 due to incorrect API route path.
- **Plate Solving Error Messages**: Improved error messages for failed plate solving jobs with actionable context (e.g., incorrect scale hints, expired jobs).
- **Plate Solving Null Jobs**: Smarter handling of null Astrometry.net jobs — distinguishes between still-processing and truly failed submissions.
- **Notification Badge**: Header notification badge now updates instantly when alerts are acknowledged in admin.
- **Image Deletion Cascade**: Deleting an image now properly cascades to plate solving jobs, equipment links, and acquisition entries.
- **Sync Metadata Errors**: Sync-metadata errors now show actionable messages instead of generic failures.

### Added
- **Astrometry.net Links**: Plate solving job details now include direct links to Astrometry.net submission and annotated result pages.
- **Gallery Auto-Refresh**: Gallery automatically refreshes after a successful Immich sync.

### Changed
- **CI**: Removed `latest` tag from main branch Docker builds.
- **Sync Error Messages**: Improved error messages during metadata sync to Immich.

## [0.5.0] - 2026-02-28

### Added
- **Equipment Catalog & Management**: Full equipment system with type-specific specification fields (focal length/aperture for telescopes, sensor type/pixel size for cameras, etc.) and custom field support.
- **Per-Filter Acquisition Tracking**: New acquisition editor for recording sub-exposure details per filter, including frame count, exposure time, gain/ISO, binning, sensor temp, and date.
- **Auto-Computed Image Summaries**: Total integration time, frame count, and filter lists are automatically computed from acquisition entries and linked equipment.
- **Location Management**: Interactive map picker for managing imaging locations.
- **Advanced Search Filters**: New search and filtering capabilities in the UI.

### Changed
- **Immich Sync**: Replaced multi-endpoint fallback sync with paginated `/api/search/metadata` for reliable full-library sync across all Immich versions.
- **Tag Filtering**: Extracted shared tag filtering logic into a reusable module; plate solving tags are now filtered consistently across sync, XMP sidecars, and the tags API.
- **Equipment Manager**: Replaced generic key/value specs with structured fields per equipment type, plus custom field support.
- **Technical Details**: Replaced the manual technical details editor with the acquisition editor for structured per-filter data entry.
- **Database Schema**: Updated to support locations, acquisitions, and enhanced metadata; migrations run automatically on startup.
- **Themed UI Components**: Replaced native select elements with themed Shadcn components.

### Fixed
- **Immich Sync Pagination**: Library sync no longer silently caps at 5000 assets; properly paginates through all results.
- **Immich Non-Album Sync**: Resolved sync issues when not using album-based sync.
- **Equipment Settings Removal**: Fixed a bug where removing image-specific equipment settings would not persist after reload.
- **Acquisition Save Errors**: Added visible error messages when saving acquisition entries fails.

### Security
- **Rollup Dependency**: Overrode Rollup to v4.59.0 to address security vulnerabilities.
- **Minimatch ReDoS**: Overrode minimatch to >=10.2.3 to fix ReDoS vulnerability from combinatorial backtracking.

## [0.4.1] - 2026-02-26

### Added
- **Visual Image Renaming**: Ability to set a custom display title for images without changing the underlying system filename.
- **PUID/PGID Support**: Support for remapping the container user ID and group ID to match host volume permissions (standard for Unraid and Linux environments).

### Changed
- **Themed UI Components**: Replaced native select and input elements with themed Shadcn components for a consistent visual identity.

### Fixed
- **Permissions**: Resolved `EACCES` errors when creating date-organized sidecar directories in Docker by supporting UID/GID remapping.

### Security
- **Rollup Dependency**: Overrode Rollup version to v4.59.0 to address multiple security vulnerabilities.

## [0.4.0] - 2026-02-26

### Added
- **Interactive Sky Map**: A high-fidelity celestial atlas powered by Aladin Lite v3 for visualizing your plate-solved image collection.
- **Image Deep Linking**: Added `?image=ID` URL parameter support to open specific image overlays directly from the sky map or shared links.
- **Pleiades (M45) Test Data**: Added high-accuracy coordinate data for Pleiades to development seed scripts.
- **Automatic GHCR Pruning**: Weekly GitHub Action to automatically clean up old or untagged Docker images from the container registry.

### Changed
- **Aladin Lite v3**: Upgraded to the latest Aladin Lite engine for improved performance and WebGL2 support.
- **Coordinate Precision**: Standardized internal coordinate storage to decimal degree strings for accurate map plotting.
- **Navigation Cleanup**: Removed unimplemented "Collections" link from the sidebar.

### Fixed
- **WebGL Detection**: Added browser capability detection with user-friendly error messages for unsupported environments.
- **Global UI Tweaks**: Added `cursor: pointer` to all button elements for better interactivity feedback.

## [0.3.0] - 2026-02-25

### Added
- **XMP Sidecar Generation**: Automatic generation of astronomical metadata sidecars for plate-solved images.
- **Date-based Organization**: Option to organize sidecar files in `YYYY-MM/` subdirectories.
- **Sidecar Download API**: Endpoint to download generated XMP files directly from the gallery.
- **Admin Configuration**: New settings panel for XMP sidecar output paths and organization rules.

### Changed
- **EXIF Extraction**: Improved lens and telescope detection from Immich metadata to auto-populate equipment fields.
- **Worker Robustness**: Enhanced plate solving worker with better error recovery and continuous polling logic.

## [0.2.0] - 2025-07-06

### Added
- **Docker Multi-stage Builds**: Optimized production-ready container images with multi-architecture support.
- **UnRAID Integration**: Dedicated templates for easy deployment on home servers.
- **PostgreSQL Support**: Full support for production-scale databases while maintaining SQLite for development.
- **Worker Management**: Fine-grained control over the plate solving background process via the UI.
- **Health Checks**: Added container health check endpoints for monitoring.

### Changed
- **Monorepo Structure**: Reorganized codebase into `/apps/` and `/packages/` for better maintainability.
- **Configuration Service**: Enhanced to prioritize database settings over environment variables.

### Fixed
- **Thumbnail Proxy**: Fixed 500 errors by ensuring proper configuration service integration.
- **Security Vitals**: Removed all hardcoded secrets and transitioned to secure environment variables.

### Security
- **SSRF Protection**: Added protocol and host validation for all external API integrations.
- **Non-root Execution**: Transitioned Docker containers to run as a dedicated user.

## [0.1.0] - 2025-01-27

### Added
- **Initial Release**: Core functionality including Immich synchronization and image management.
- **Plate Solving**: Astrometry.net integration for automatic celestial coordinate detection.
- **Equipment Catalog**: Manage telescopes, cameras, and accessories.
- **Deep Zoom Viewer**: OpenSeaDragon integration for high-resolution exploration.
- **Real-time Updates**: WebSocket (Socket.io) support for live processing status.
- **Cron Scheduling**: Automated synchronization tasks.
