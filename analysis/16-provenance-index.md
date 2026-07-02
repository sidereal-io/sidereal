# 16. Provenance Index

Map from each evidence ID to the claims, specs, schemas, test vectors, and acceptance criteria derived from it. (Sections: §3 surface map, §4 domain, §5 specs, §6 API, §7 UI, §8 workers, §9 persistence, §10 errors; TV = test vectors, AC = acceptance criteria, C = conflicts, Q/U = open questions.)

## Source inspection

| Evidence | Derived claims / artifacts |
|---|---|
| SRC-01 | SPEC-SYS-1, SPEC-SYS-2; §3.1 SPA fallback, §3.5 startup jobs, §8.5; TV-001/002/005/006; AC-1.1–1.4, AC-2.3 |
| SRC-02 | SPEC-SYS-1 (DB selection/migration); §9.1, §9.4; TV-001; AC-1.2, AC-10.2; C12 |
| SRC-03 | SPEC-IMG-1/2/3, SPEC-EQP-2/4 (routes), sidecar fetch; §6 Images table; §10.2/10.4; TV-030–036, TV-041–044, TV-066; AC-3.4 |
| SRC-04 | SPEC-CFG-1/2/3, SPEC-STAT-1, SPEC-NTF-1, SPEC-ADM-1/2, SPEC-SYS-3; §10.2–10.5; TV-004/010–014/080–082/090–092; AC-2, AC-8; C1, C10 |
| SRC-05 | §3.2 Immich endpoints; SPEC-SYNC-1/2 routes, SPEC-CFG-3; C2; TV-070/075 |
| SRC-06 | SPEC-SOLVE-1/3 (routes); §6 plate-solving; TV-060/061/065; AC-5.1 |
| SRC-07 | SPEC-CAT-3/4 (thumbnail, backfill-targets); §3.6 CDS integration, §8.6; TV-058; AC-6.4/6.6 |
| SRC-08 | SPEC-EQP-1; §6 equipment; TV-040/046; AC-7.1; C8 |
| SRC-09 | SPEC-EQP-3; §6 groups; TV-045; AC-7.4; C8 |
| SRC-10 | SPEC-LOC-1; §6 locations; AC-7.5; C7, C8 |
| SRC-11 | SPEC-TGT-1 (routes/filters); §6 targets |
| SRC-12 | SPEC-TGT-2; §6 user-targets; TV-057; C8 |
| SRC-13 | §3.2 sky-map markers; §7.5; TV-106 |
| SRC-14 | SPEC-IMG-4/5 (routes/validation); §6 sources; TV-020–024 |
| SRC-15 | SPEC-PRX-1; §6 asset proxy; AC-4.6; §10.3 (503) |
| SRC-16 | §10.1 universal error shape/status forwarding; AC-2.2 |
| SRC-17 | SPEC-CFG-1/2 (defaults, precedence, caching, XMP_SIDECAR_PATH override); §3.4; C9; AC-10.5 |
| SRC-18 | SPEC-SOLVE-1/2/3; §4.6, §8.2, §13.8; TV-062/063; AC-5.2–5.4; C13 |
| SRC-19 | SPEC-JOB-1, cron registration/failure notifications, SPEC-SYNC-1 scheduled path; §8.3; TV-080/095; AC-4.3, AC-8.1; C1, C10, Q5 |
| SRC-20 | SPEC-CAT-1/2/4; §4.4, §8.5, §13.11; TV-050–055/058; AC-6.1–6.4 |
| SRC-21 | Persistence semantics: filters, cascade delete, settings upsert, notifications, recompute, stats; §9.2/9.3; TV-013/043/046/057; AC-3.7, AC-7.3; C8 |
| SRC-22 | SPEC-IMG-6, orphan-sweep mechanics; §9.3 filesystem-as-state, §13.4; TV-020/095; AC-3.1/3.5 |
| SRC-23 | SPEC-SOLVE-4; §13.4 sidecar layout; TV-066; AC-5.6; Q10 |
| SRC-24 | SPEC-SOLVE-5 (prod supervision), health worker block; §8.2 ops row; AC-1.5 |
| SRC-25 | SPEC-WS-1 (envelope, broadcast, path); §3.3; TV-084/085; AC-8.3; Q4 |
| SRC-26 | Tag relevance rules in SPEC-SOLVE-2/STAT-1; TV-083; §13.8 |
| SRC-27 | §3.2 source registry (three types); API-07 corroboration |
| SRC-28 | SPEC-SYNC-1 (algorithm, mapping, reconciliation, size cap); §4.1 ingestion defaults, §13.7; TV-071–074; AC-4.1/4.2; C7 |
| SRC-29 | SPEC-IMG-4 (hash dedup, rollback); TV-021/025; AC-3.2/3.6 |
| SRC-30 | SPEC-IMG-5 (URL hash, ext inference); TV-023/024; Q7 |
| SRC-31 | SPEC-SOLVE-5 (loop, standalone mode, WS relay); §3.4 worker env vars, §8.7; TV-064; AC-5.5; C9, Q4 |
| SRC-32 | SPEC-SYNC-2 (candidates, single-flight, batching, events); §8.4; TV-075/076; AC-4.5 |
| SRC-33 | §7 route list, dark theme; §3.1 |
| SRC-34 | §3.3 client reconnect/backoff and event subscriptions; §7.0; AC-8.3 |
| SRC-35 | §7.0 data-layer behavior (no retry/refetch, filter→params); §10.8 |
| SRC-36 | §7.1 gallery workflow, deep-linking, load-more, client-side filters |
| SRC-37 | §7.2–7.7 page↔endpoint dependency mapping (header sync button, admin fetches, bulk solve, markers) |
| SRC-38 | SPEC-EQP-4 recompute contract, SPEC-TGT-1 grouping/enrichment/sort; §4.5; TV-042/056; AC-7.3, AC-6.3 |
| SRC-39 | Name normalization / RA-Dec conversion / CSV parsing / match priority in SPEC-CAT-1/2/4; §13.11 |
| SRC-40 | §7.2 client-side visibility hints (best month, circumpolar) |
| SRC-41 | §13.1 image URL scheme; TV-102; AC-3 preamble |

## Schemas, migrations

| Evidence | Derived |
|---|---|
| SCHEMA-01 | §4 field inventories; §9.2 tables/uniqueness/cascades; C13; AC-10.1 |
| SCHEMA-02 | §9.1 dialect parity claim; AC-10.2 |
| SCHEMA-03 | C11 (legacy module disregarded) |
| SCHEMA-04 | §4.5 equipment types/spec fields, §4.8 notification shape; TV-104 (type options) |
| SCHEMA-05 | §4.2 source status/ingest result shapes; §6 SourceStatus/IngestResult |
| MIG-01 | §9.4/9.5 migration history; C5 (`immich_id` drop); AC-1.2, AC-10.1 |

## Tests

| Evidence | Derived |
|---|---|
| TEST-01 | §7.1 assertions; TV-100/101 |
| TEST-02 | §7.7 admin form contract; TV-105 |
| TEST-03 | §7.1 image-source rule, cache header; TV-102; AC-9.2 |
| TEST-04 | §7.8/7.9 degraded rendering + 404 page; TV-103/108; AC-9.3 |
| TEST-05 | §7.0 header consistency, GitHub URL, active-link styling; C14 |
| TEST-06 | §7.3 equipment form gating; TV-104 |
| TEST-07 | §7.4 five stats cards, selection UI |
| TEST-08 | §7.5 sky-map states/navigation; TV-106 |
| TEST-09 | §7.7 notification card rules; TV-107; AC-9.1 |
| TEST-10 | §13.1/§13.10 imageUrl contract (executed, RUN-02) |
| TEST-11 | noted corroboration for serving/image-storage/source specs (not executed) |

## Documentation

| Evidence | Derived |
|---|---|
| DOC-01 | §6 doc-drift subsection; C1–C9, C13; §10.6 "no auth by design"; §13.2 warning |
| DOC-02 | §3.10 commands; C10; §3.4 env table cross-check |
| DOC-03 | §9.6 rename compatibility; §13.5 volume/db naming; C14 |
| DOC-04 | §3.4 env surface; §13.6 worker standalone vars; C9 |

## Docker

| Evidence | Derived |
|---|---|
| DOCKER-01 | SPEC-DCK-1 (image, healthcheck, dirs, user); AC-10.4 |
| DOCKER-02 | SPEC-DCK-1 entrypoint semantics; §9.4 PG best-effort migrations; §9.6 auto-migrate; TV-093/094; AC-10.4 |
| DOCKER-03 | §13.5 volumes/env; SIDEREAL_PORT; external Immich volume |
| DOCKER-04 | §9.1 PG option; TV-092 context; AC-10.2 |

## Runtime

| Evidence | Derived |
|---|---|
| RUN-01 | §1 quality-gate state |
| RUN-02 | §13.10 unit contract; TV note |
| RUN-03 | SPEC-SYS-1 startup order; §8.3 schedule registration; §8.4 no-op backfill; TV-002 |
| RUN-04 | SPEC-SYS-1 fresh start; SPEC-CAT-1 auto-load (14,033); §9.4; TV-001/055; AC-1.1–1.3 |
| API-01 | SPEC-SYS-3; TV-003; AC-1.5 |
| API-02 | SPEC-STAT-1 exact shape; TV-082; C4 |
| API-03 | SPEC-CFG-1 defaults doc; TV-010 |
| API-04 | SPEC-ADM-2 broken endpoint; C1; AC-11a |
| API-05 | SPEC-ADM-1; TV-090 |
| API-06 | SPEC-CAT-1 status shape; TV-055; C3 |
| API-07 | §3.2 sources list shape; §4.2; Q12 |
| API-08 | SPEC-IMG-1/2 (empty list, 404s, non-numeric id); SPEC-SYS-2 API 404; TV-005/036 |
| API-09 | SPEC-CAT-2/3 live shapes (`pageSize`, object fields, alias search); TV-050–052; C3 |
| API-10 | §10.2/10.3 observed messages+codes (settings, tests, solve, sources, backfill 200); TV-012/014/022/023/060/061/070/075 |
| API-11 | SPEC-IMG-4 (201, dedup), §4.1 local defaults; TV-020/021 |
| API-12 | SPEC-IMG-3 full header/ETag/range/backfill contract; TV-030–034; AC-3.4 |
| API-13 | SPEC-EQP-1/2/4 recompute numbers, SPEC-IMG-2 PATCH, SPEC-TGT-1 enrichment, SPEC-TGT-2 upsert, SPEC-LOC-1; TV-040–044/056/057; AC-7.3 |
| WS-01 | SPEC-WS-1 connection; TV-084 |
| FS-01 | SPEC-IMG-6 layout (`processed/001/1/…`); §13.4; TV-020 |
