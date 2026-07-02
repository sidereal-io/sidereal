# 9. Persistence and Migration Semantics

What a replacement must preserve about *data at rest* and upgrades. Table/column names are listed only because they are a **data-compatibility surface** (existing SQLite/PG databases must keep working); they are not a design prescription.

## 9.1 Storage engines

- **Default**: single-file SQLite. Path: `SQLITE_DB_PATH` env → else `local.db` (dev) / `/app/config/sidereal.db` (container). Created on first start. (SRC-02, RUN-04, DOCKER-02)
- **Optional**: PostgreSQL, selected solely by the presence of `DATABASE_URL`. (SRC-02)
- Behavioral parity is intended between dialects (schemas maintained in lockstep; tag filtering implemented per-dialect: PG array-overlap vs SQLite JSON containment). PG mode was **not** runtime-verified (Unknown U1). (SCHEMA-01/02, SRC-21)

## 9.2 Persisted entities (user-visible semantics)

| Entity (table) | Key semantics & uniqueness | Delete behavior (observed contract) |
|---|---|---|
| Image (`astrophotography_images`) | Auto-increment `id` (embedded in URLs and storage paths — **ids must be preserved on data migration**); logical dedup key `(source_type, source_id)`, not DB-enforced | Deleting an image removes its plate-solving jobs, equipment links, acquisition entries, and its on-disk rendition directory (file cleanup best-effort/non-fatal) |
| Equipment (`equipment`) | id | Cascades: image links and group memberships removed; acquisition `filter_id` set null |
| Image–equipment link (`image_equipment`) | id; logical pair (imageId, equipmentId), not DB-enforced unique | — |
| Acquisition (`image_acquisition`) | id; `image_id` required | removed with image |
| Plate-solve job (`plate_solving_jobs`) | id; references image | removed with image |
| Admin setting (`admin_settings`) | `key` UNIQUE; JSON value | upsert on write |
| Notification (`notifications`) | id | acknowledged ones auto-purged after 30 days |
| Equipment group (`equipment_groups`) + members (`equipment_group_members`) | id | group delete removes memberships only |
| Catalog object (`catalog_objects`) | `name` UNIQUE | full-replace on catalog reload |
| Location (`locations`) | id | standalone |
| User target (`user_targets`) | `catalog_name` UNIQUE | standalone |

Timestamps: SQLite stores epoch timestamps; API serializes ISO-8601. `created_at`/`updated_at` auto-set; `updated_at` bumped on updates (images, locations, groups, user targets). (SCHEMA-01, API-11/13)

## 9.3 Cross-cutting persistence rules

- **Derived-field recompute**: image summary fields are recomputed from acquisitions + linked equipment on every change to either (SPEC-EQP-4) — a migration/replication must treat those image fields as *materialized derived state*, not independent truth (though they may also hold source-derived EXIF values for images with no acquisitions).
- **Soft references**: `target_name` and `catalog_name` are strings matched by name/Messier designation; catalog reloads can orphan them harmlessly.
- **Settings document**: stored as one row per top-level section key (`immich`, `astrometry`, `sidecar`, `app`) plus scalar catalog metadata keys (`catalog_lastUpdated`, `catalog_commitSha`, `catalog_objectCount`). API keys stored **in plaintext** in the DB (privacy consideration; also means DB backups contain secrets). (SRC-21, SRC-17)
- **Filesystem is co-equal persisted state**: `{STORAGE_PATH}/processed/...` renditions, sidecar `.xmp` files, and the DSS thumbnail cache. DB backup alone does not capture images; the orphan sweep deletes image dirs with no DB record (restoring an older DB can therefore trigger deletion of newer images' files at 03:00 — operational hazard worth documenting). (SRC-22, SRC-19)

## 9.4 Migration-on-startup behavior

- SQLite: pending migrations from a journaled, ordered series are applied automatically at boot before serving; already-applied ones are skipped; failure is fatal. Observed: fresh DB → all 12 applied silently ("Migrations completed successfully"). (SRC-02, RUN-04, MIG-01)
- PostgreSQL (container): a migration script runs at container start, **failures logged but startup continues** (a schema-behind PG database can serve — degraded-mode hazard). (DOCKER-02)
- Downgrade behavior: none defined anywhere. Unknown (U2).

## 9.5 Migration history (what upgrades did, behaviorally)

From the SQLite series (MIG-01), the externally meaningful upgrades an existing deployment has been through:
- `0004`: images gained `original_path` (local storage era begins).
- `0005`: equipment gained `cost`, `acquisition_date`.
- `0006`: equipment groups introduced.
- `0008`: images gained `target_name` (targets feature).
- `0009`: local image storage (backfill-era support).
- `0010`: images gained `source_type` (default `'immich'`) and `source_id`; existing Immich ids copied into `source_id`.
- `0011`: `immich_id` column dropped — **`(source_type, source_id)` is now the only provenance key**. Any external consumer still expecting `immichId` breaks here (C5).

## 9.6 Cross-database data migration (upgrade tooling)

- A migration CLI (`migrate-db`) copies data between any two of {SQLite path, PostgreSQL URL}; the container automates it one-time via `AUTO_DB_MIGRATE_FROM`/`AUTO_DB_MIGRATE_TO` with a marker file (`/app/config/.auto-db-migrated`), optional pre-wipe of the SQLite target, and abort-on-failure. (DOCKER-02, tools/scripts inventory; internals not audited — Medium confidence.)
- Rename compatibility (Skymmich→Sidereal): documented manual steps for volume renames, `skymmich.db`→`sidereal.db`, PG db/user renames; `SQLITE_DB_PATH` overrides make the filename non-contractual. (DOC-03)

## 9.7 Backup/restore surface

- SQLite: `GET /api/admin/database/backup` streams the live DB file (no quiesce/checkpoint step observed — restore consistency under concurrent writes is Unknown U3). PG: explicitly out of scope (400 + pg_dump advice). (SRC-04)
