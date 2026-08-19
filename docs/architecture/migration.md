# Migration & Cutover

**Tracks:** [RFC #213](https://github.com/sidereal-io/sidereal/issues/213) · **Last updated:** 2026-08-19 ·
Part of the [architecture reference](README.md).

v2 is a new data model. The strategy is a **clean break with a one-way importer**, gated on a
non-negotiable feature set — the decision and its rationale are
[ADR-010](../decisions/ADR-010-migration-strategy.md); this doc owns the execution. The TypeScript app
moves to maintenance (security and critical fixes only) and is retired at cutover. The importer reads an existing SQLite/Postgres database and storage tree and
produces v2 assets — best-effort, lossy where the models genuinely differ, and it **emits a report of
exactly what didn't map**.

## On the analysis package's compatibility requirements

`13-compatibility-requirements.md` states a replacement MUST preserve image IDs, the storage layout
`{STORAGE_PATH}/processed/{id % 1000}/{id}/…`, every API path and shape, and reconciling-sync semantics.

**v2 does not honour that, and cannot.** Those requirements describe a system where Immich owns the tree
and an `Image` is a finished photo — both premises are exactly what v2 exists to change. The analysis
package stays valuable as a behavioural inventory and as the source of the cutover gate below, but it is
not a contract v2 is bound by.

## Non-negotiable before cutover

An existing user would consider the upgrade broken without these:

- [ ] Gallery browse / filter / search with deep links
- [ ] Image detail view and metadata editing
- [ ] Deep-zoom viewer
- [ ] Plate solving, single and bulk
- [ ] Targets: catalog browse, visibility, annotations
- [ ] Equipment and equipment groups
- [ ] Acquisition entries and integration totals
- [ ] Immich sync as a source
- [ ] Admin configuration UI with connection tests
- [ ] Docker parity — port 5000, volume mounts, PUID/PGID, healthcheck
- [ ] Saved locations and their session relationships
- [ ] One-way importer from v0.10.x with dry-run and reconciliation report

**Should-have, not blocking:** sky map with FOV overlay · notifications · live job updates · dashboard
stats.

**Compatibility breaks pending a deployment survey/data scan:** XMP sidecar generation · standalone
worker mode. Current evidence shows uncertain lifecycle or usage, not absence of users. Survey
deployments and inspect available configuration/telemetry before accepting the break; if dropped, the
importer reports affected records explicitly.

**Explicitly dropped:** the database-download API endpoint (unauthenticated full-database download;
replaced by documented volume backup) · legacy free-text `telescope`/`camera`/`mount` fields (superseded
by equipment relations).

## Filesystem safety

M1–M5 builds operate only on copied or disposable storage roots. They never rename, move, or delete
irreplaceable originals. The importer is read-only against the source tree, supports dry-run and resumable
execution, records legacy-ID mappings, verifies checksums, and reconciles source/destination counts. Its
hard invariant: every irreplaceable local or URL original is either imported and verified or named in the
failure report — an unaccounted original blocks cutover.

## Rollback

- **Before M6:** discard the disposable v2 root; v0.10.x and its source tree remain untouched.
- **At M6:** the migration guide requires a verified database and storage backup before import.
- **After cutover:** rollback restores that backup. The importer is one-way by design.

Because ADR-004 accepts PostgreSQL-only, backup guidance covers the Postgres volume (and `pg_dump`) in
place of the old SQLite-file copy.
