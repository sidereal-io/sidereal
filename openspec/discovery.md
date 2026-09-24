# Discovery: M1 — Core spine & first plugins

> Status: complete
> Created: 2026-09-18 · Last revised: 2026-09-20

> Release plan produced by the discovery skill. Resume or revise by re-running the skill.
> To build: run `/opsx:propose` and ask it to use the next unchecked story below.
> One story = one OpenSpec change (proposal ≈ 200 words). Create one at a time.

## Sources

- 2026-09-18 — GitHub issue [#217](https://github.com/sidereal-io/sidereal/issues/217) "M1: Core spine & first plugins" (body + Mike's review comment), the [v2 roadmap](../docs/architecture/roadmap.md), and the M0 backend (`backend/` cargo workspace: `plugin-abi`, `core`, `packs/astro`, `server` serving `GET /healthz`).

## Scope (restated)

**What M1 proves:** the plugin architecture survives contact with reality. One real vertical path — a file dropped in a watched folder becomes a managed, content-hashed, metadata-tagged asset visible in a new UI — runs **entirely through the same `plugin-abi` contract a third party would use**. No private core back door for built-ins.

**The exit test, stated once:** in a disposable storage root, drop a file in a watched folder → it appears in the UI with extracted metadata. That single path, automated in CI, *is* the milestone.

### Goals

- Persist the domain model in PostgreSQL: Asset (envelope = id/kind/name), immutable content-addressed `AssetVersion`, Operation Run + event log, core-owned JSONB facets. Collection and Lineage as schema-only scaffolding.
- Own files on disk: managed store, BLAKE3 hashing, atomic ingest by copy, dedup, and filesystem-integrity reconciliation (verify + adopt/restore/ignore).
- A minimal single-worker executor: durable `OperationRun` records, a durable event log, WebSocket progress fan-out, cooperative cancellation.
- Grow `plugin-abi` from id-stubs to a working (unfrozen) Source/Operator contract with a minimal read-only `AssetContext`, plus a new domain-agnostic `builtins` crate.
- Two plugins over that contract: a watch-folder Source and a generic metadata-extraction Operator.
- The first read-only screen of the new ADR-005 frontend shell.

### Non-goals (deferred)

- No Selector / Goal / Policy engine; M1 dispatch is one hardcoded rule (`asset.ingested → run metadata operator`). → M2
- No retries, concurrency limits, missed-event sweep, or crash resumption. → M2
- No manifest, config schema, capability grants, embedded-script profile, conformance suite, or ABI freeze. → M2
- No FITS/XISF parsing — generic file metadata only. → M3
- No rename/move operators or user-meaningful storage tree. → M2/M3
- No auth/CORS hardening — left stubbed-open with the seam in place (ADR-007).
- No SQLite / dual-dialect — PostgreSQL only, one forward-only migration chain (ADR-004).

## Personas

### Pat — the plugin author

- **Who**: builds a Source or Operator against `plugin-abi`. In M1 this is a Sidereal maintainer *wearing the third-party hat* — writing the two built-ins with no privileges a real third party wouldn't have.
- **Goal**: make a file-discovering Source and a metadata-emitting Operator work end to end through the public contract alone.
- **Pain today**: `plugin-abi` is id-stubs — no `AssetContext`, no byte access, no facet emission, and no proof the contract is expressive enough for real work.
- **Success looks like**: both plugins run through the same surface a stranger would use, and any attempt to reach around it (direct store write, forged facet) is rejected.

### Nova — the astrophotographer

- **Who**: the end user who will eventually manage thousands of frames; in M1 she gets the first thin taste.
- **Goal**: drop a file into a folder and see Sidereal recognize it.
- **Pain today**: v2 turns nothing into a managed asset — M0 only serves `/healthz`.
- **Success looks like**: a file appears in the UI on its own, with size/format/dimensions filled in, within seconds of landing in the folder.
- *Honest scope*: M1 is a sliver of Nova's journey — generic metadata only, read-only UI, no astro. Her real payoff is M3.

### Sam — the self-hoster / operator

- **Who**: runs the Sidereal box on their own hardware.
- **Goal**: stand it up in roughly one command and trust it with their files.
- **Pain today**: nothing to run; no storage story; no assurance originals are safe.
- **Success looks like**: Postgres + app come up bundled; originals in the watched folder are never moved or altered; and if a stored file is corrupted, Sidereal detects it and offers a fix (adopt/restore/ignore).

## Journey Map

M1 is greenfield, so most stages are gaps. M0's one solid rung is the **registration path** (`Pack::register` + `Registry`). Nova's spine can't be reordered much — no "see in UI" without ingest, and ingest is gated on ADR-011.

**Nova's spine — the asset path (this *is* the exit criterion):**

```
  Run app ──► Point at ──► Drop ──► Ingest ──► Extract ──► See in ──► Live
  (Postgres)   folder      file    (hash/       metadata    UI        update
                                    store/ver)                         (WS)
     │           │           │        │            │          │          │
   partial     gap         gap      gap          gap        gap        gap
```

1. **Run app** — bundled Postgres + app boot — *partial* (Dockerfile + axum shell exist; no DB, no compose bundle)
2. **Point at a watched folder** — config/env — *gap*
3. **Drop a file, auto-detected** — Source + `notify` + debounce — *gap*
4. **Ingest** — copy, BLAKE3 hash, dedup, store, mint `AssetVersion` — *gap* (no schema, no store)
5. **Extract metadata** — executor runs the Pure operator, emits `core.*` facets — *gap*
6. **See in UI** — read-only asset list + detail — *gap* (frontend is v0.10.x; no v2 shell)
7. **Live update** — `asset.ingested` over `/ws` — *gap*

**Pat's overlay — build a plugin through the contract:**

```
  Code to ──► Get ──► Read bytes ──► Emit ──► Register ──► Run via ──► Back-door
  contract   AssetCtx   /facets    facet     plugin      executor    rejected
     │          │          │          │         │            │           │
  partial     gap        gap        gap    supported       gap         gap
```

- **Code to contract** — *partial* (Source/Operator traits exist, id-only)
- **AssetContext** (byte read, `emit_facet`, `log`, `is_cancelled`) — *gap*
- **Register plugin** — *supported* (`Pack::register` + `Registry` proven in M0)
- **Run via executor / outcome validated / no back door** — *gap*

**Sam's overlay — install and trust:**

```
  Install ──► Configure ──► Originals ──► Detect ──► Repair
  (1 cmd)                   untouched    corruption  (adopt/restore/ignore)
     │           │             │            │            │
  partial       gap           gap          gap          gap
```

- **One-command install** — *partial* (Dockerfile exists; no bundled-Postgres compose)
- **Filesystem safety** (copy, never move) — *gap*
- **Integrity detect + repair** — *gap*

## MoSCoW

**Gate above all Musts:** ADR-011 (Proposed) must be Accepted before the store/ingest/integrity work — a decision record, not a build item.

### Must — required for the exit criterion (drop file → appears in UI with metadata, all through contracts)

- **Schema + migrations + core types** — one forward-only PostgreSQL chain (Asset, `AssetVersion`, `operation_run` + event log, JSONB facets + GIN; Collection/Lineage tables as scaffolding to avoid a later migration). Nothing persists without it.
- **Managed store + atomic ingest** — copy, BLAKE3, dedup, path-safety. Nova stage 4; Sam's originals-untouched. Gated by ADR-011.
- **`plugin-abi` growth + `builtins` crate** — working `Source`/`Operator`/`AssetContext`/`OperatorOutcome`, arch-lint extended. Pat's entire journey.
- **Executor + events + `/ws` + hardcoded dispatch + cooperative cancel** — durable `operation_run` lifecycle, broadcast bus. Nova stages 5 & 7.
- **WatchFolderSource** — `notify` streaming + rescan backstop + debounce. Nova stages 2–3.
- **MetadataOperator + `core.*` facets** — the Pure operator. The "with metadata" in the exit test.
- **Outcome validation / no-back-door enforcement** — facets only via `emit_facet`; direct store writes rejected. Pat's success signal — the milestone's soul.
- **HTTP API** — assets, versions/`current`, by-hash, runs. Nova stage 6; observability for all three personas.
- **Read-only UI** — first screen of the ADR-005 Option-C shell (asset list + detail, live over `/ws`). Literally "appears in the UI."
- **Integration/E2E exit-path test + CI (with Postgres service)** — this test *is* the exit criterion, automated.

### Should — hardens trust, but M1 exits without it

- **Filesystem-integrity reconciliation** (verify + adopt/restore/ignore) — moved down from the issue's Goals: it's Sam's trust journey, off Nova's exit path, and carries the unresolved "adopt = second version" question. (Confirmed with product owner 2026-09-20.)
- **Bundled-Postgres image/compose (one-command install)** — Sam stage 1. The CI exit test only needs *a* Postgres; the polished bundle can trail the path.

### Could

- **Collection / Lineage minimal API** — the tables ride along in the schema Must; exposing them over HTTP has no consumer in M1.
- **Playwright smoke** against the minimal SPA.

### Won't (this release)

Deferred, with the milestone that owns each: Selectors / Goals / Policies (M2) · retries, concurrency limits, missed-event sweep, crash resumption (M2) · manifest, config schema, capability grants, embedded-script profile, conformance suite, ABI freeze (M2) · FITS/XISF parsing (M3) · rename/move operators + user-meaningful tree (M2/M3) · auth/CORS hardening (ADR-007) · SQLite / dual-dialect (ADR-004).

## Stories

Ordered release checklist. One story = one OpenSpec change (proposal ≈ 200 words).
Every build story is a thin vertical slice — end-to-end and demoable. The issue's
steps 0–10 were horizontal layers; these are re-cut vertically so the whole spine is
proven early (story 2) and then thickened, rather than 8 layers before anything runs.

- [ ] 1. `accept-adr-011-storage-tree` — ratify the storage layout so storage work can start
  - **Persona served**: Sam (indirectly all)
  - **Journey segment**: gate above Nova stage 4 (Ingest)
  - **MoSCoW**: Must (prerequisite gate — a decision record, not a vertical slice)
  - **Why this story / why now**: ADR-011 is Proposed and, per the STOP-on-open-ADR rule, gates every store/ingest/on-disk section. Nothing storage-related may be designed until it is Accepted. Work it with the operator to Accepted: content-addressed internal store, ingest protocol, GC, cross-filesystem moves.
  - **Depends on**: nothing
  - **Scope**: in — drive `ADR-011` to Accepted (tree layout, atomic-ingest protocol, refcount GC, move protocol) / out — any application code; that lands in story 2+.
  - **Relevant code**: `docs/decisions/ADR-011-storage-tree-layout.md`; `docs/architecture/migration.md` (filesystem-safety rule)
  - **Added**: 2026-09-20
  - **Change**: _not yet proposed_

- [ ] 2. `ingest-file-to-visible-asset` — a file on disk becomes a managed, hashed asset you can GET ⭐ walking skeleton
  - **Persona served**: Nova (thin taste) + foundation for every later story
  - **Journey segment**: Nova stages 4 & 6 (Ingest → See), the thinnest path through them
  - **MoSCoW**: Must
  - **Why this story / why now**: the walking skeleton — the thinnest end-to-end path that turns a file into a persisted, queryable managed asset. Core owns ingest permanently (the Source only feeds paths later), so a manual/dev trigger here is not throwaway. Everything else thickens this.
  - **Depends on**: story 1 (ADR-011 Accepted)
  - **Scope**: in — first forward-only PostgreSQL migration (`asset`, `asset_version` incl. `version_seq`, `content_hash`, `storage_path`); content-addressed managed store (copy, BLAKE3, `fsync`, atomic rename, dedup-by-hash, path-safety rejecting `..`/symlink escape, startup orphan sweep covering `store/` as well as `tmp/`); a core ingest entry point invoked by a dev/test trigger; read API `GET /api/assets`, `/api/assets/:id`, `/api/assets/by-hash/:hash`, `/api/assets/:id/versions/current`. / out — watch-folder, operators, executor, facets, UI, integrity path.
  - **Relevant code**: `backend/crates/core/src/lib.rs` (ingest + store live here); `backend/crates/server/src/lib.rs` (`app()` router); new `sqlx` migrations dir; `backend/Cargo.toml` (add `sqlx`)
  - **Added**: 2026-09-20
  - **Change**: _not yet proposed_

- [ ] 3. `operator-run-through-contract` — an ingested asset triggers the first plugin, through `plugin-abi`, and gains a facet
  - **Persona served**: Pat (this is the milestone's soul: the plugin contract meets reality)
  - **Journey segment**: Nova stage 5 (Extract) + 7 (Live); Pat's whole overlay
  - **MoSCoW**: Must
  - **Why this story / why now**: proves an operator can do real work through the same public surface a third party would use, with no private back door — the one thing M1 exists to de-risk. Kept to a single observable facet so the proposal stays inside budget; story 4 adds the rest.
  - **Depends on**: story 2
  - **Scope**: in — grow `plugin-abi` (`Operator` with `accepts`/`execute`, minimal read-only `AssetContext` with byte read + `emit_facet` + `log` + `is_cancelled`, `OperatorOutcome`); new domain-agnostic `builtins` crate with a Pure `core.metadata` operator emitting `core.file.byte_size`; single-worker executor (durable `operation_run` lifecycle, durable `operation_event` log, `tokio` broadcast bus, `GET /ws`); hardcoded `asset.ingested → run core.metadata` dispatch; cooperative cancellation; outcome validation (facets only via `emit_facet`, direct store writes rejected); `GET /api/runs`, `/api/runs/:id`, `POST /api/runs/:id/cancel`; `running→failed` on restart. / out — full metadata set (story 4), watch-folder (story 5), retries/selectors (M2).
  - **Relevant code**: `backend/crates/plugin-abi/src/lib.rs` (grow the `Operator` trait); new `backend/crates/builtins/`; `backend/scripts/check-arch.sh` (add `builtins` to the boundary lint); `backend/crates/core/src/lib.rs` (executor + dispatch); `backend/crates/server/src/lib.rs` (`/ws`, `/api/runs`); `backend/Cargo.toml` (workspace member)
  - **Added**: 2026-09-20
  - **Change**: _not yet proposed_

- [ ] 4. `metadata-operator-full-facets` — the operator fills in the real metadata set
  - **Persona served**: Nova
  - **Journey segment**: Nova stage 5 (Extract), thickened
  - **MoSCoW**: Must
  - **Why this story / why now**: story 3 proves one facet; this delivers the metadata a user actually sees. Split out so story 3's proposal stays within budget.
  - **Depends on**: story 3
  - **Scope**: in — extend `core.metadata` to emit `core.file.format`, `core.file.mtime`, and `core.image.{width,height}` for standard rasters (JPEG/PNG/TIFF) via the `image` crate. / out — FITS/XISF parsing (M3).
  - **Relevant code**: `backend/crates/builtins/` (the `core.metadata` operator); `backend/Cargo.toml` (`image` crate)
  - **Added**: 2026-09-20
  - **Change**: _not yet proposed_

- [ ] 5. `watch-folder-source` — drop a file in a folder and it ingests itself, no manual step
  - **Persona served**: Nova + Pat (the Source half of the contract)
  - **Journey segment**: Nova stages 2–3 (Point at folder → Drop)
  - **MoSCoW**: Must
  - **Why this story / why now**: replaces story 2's manual trigger with the real automated entry point, running through the `Source` contract. This is what makes the exit criterion headless.
  - **Depends on**: stories 2, 3 (parallel with story 6)
  - **Scope**: in — grow `plugin-abi` (`Source` + `SourceContext` with `emit_candidate`); `builtins` WatchFolderSource (watched dirs from env/config, `notify` streaming + periodic rescan backstop, size-stabilization debounce); `external_id` = relative path + size + mtime for `(source_instance_id, external_id)` idempotency; core ingests emitted candidates. / out — manifest/config-schema (M2).
  - **Relevant code**: `backend/crates/plugin-abi/src/lib.rs` (`Source`/`SourceContext`); `backend/crates/builtins/`; `backend/crates/core/src/lib.rs` (candidate → ingest); `backend/Cargo.toml` (`notify` crate)
  - **Known limitation**: editing a watched file in place changes its `external_id`, so it ingests as a new asset rather than a new version — accepted for M1 (see Open Questions).
  - **Added**: 2026-09-20
  - **Change**: _not yet proposed_

- [ ] 6. `read-only-asset-ui` — assets appear live in a browser
  - **Persona served**: Nova
  - **Journey segment**: Nova stage 6–7 (See → Live update)
  - **MoSCoW**: Must
  - **Why this story / why now**: the "appears in the UI" half of the exit criterion, and the first screen of the ADR-005 Option-C shell — not throwaway. Parallel with story 5.
  - **Depends on**: story 3 (needs `/ws`, facets, and the asset API)
  - **Scope**: in — new Vite/React/Tailwind app scaffold (the v2 shell); read-only asset list that live-updates on `asset.ingested` over `/ws`; detail panel (envelope, facets, versions/run status, Technical/on-disk section: id, version id, content hash, resolved storage path with copy-path, ingested-from provenance). / out — any writes; M5 parity features.
  - **Relevant code**: new v2 frontend scaffold (v0.10.x frontend is `apps/client`; v2 shell is a fresh app, location TBD); consumes the story-2/3 HTTP + `/ws` API
  - **Added**: 2026-09-20
  - **Change**: _not yet proposed_

- [ ] 7. `exit-path-e2e-and-ci` — the exit criterion, automated in CI
  - **Persona served**: all three (this test *is* the milestone)
  - **Journey segment**: the whole Nova spine, end to end
  - **MoSCoW**: Must
  - **Why this story / why now**: turns the exit criterion into a repeatable gate. Comes after the path is walkable end to end.
  - **Depends on**: stories 5, 6
  - **Scope**: in — integration test against a disposable `STORAGE_ROOT` (drop a fixture → assert `asset` + `asset_version` + `core.*` facets over `GET /api/assets/:id`, and `asset.ingested` + `operation.succeeded` over `/ws`); extend `backend-rs.yml` with a PostgreSQL service + disposable-root fixtures; container smoke test against bundled Postgres; light Playwright smoke against the SPA; contract-honesty test (direct store write rejected). / out — M2 conformance suite.
  - **Relevant code**: `.github/workflows/backend-rs.yml`; `backend/crates/server/tests/` (mirror `healthz.rs`); Playwright config for the new SPA
  - **Added**: 2026-09-20
  - **Change**: _not yet proposed_

- [ ] 8. `integrity-reconciliation` — detect a corrupted stored file and repair it
  - **Persona served**: Sam
  - **Journey segment**: Sam's Detect → Repair
  - **MoSCoW**: Should (off Nova's exit path; confirmed with product owner 2026-09-20)
  - **Why this story / why now**: hardens Sam's trust in the store. Independent of the exit path, so it can land after the spine is green. Resolves the "adopt = second version" open question.
  - **Depends on**: stories 2, 3
  - **Scope**: in — `POST /api/assets/:id/verify` (rehash on-disk bytes vs stored `content_hash`), `availability = integrity_mismatch` + `integrity.mismatch` event; `POST /api/assets/:id/integrity/resolve {adopt|restore|ignore}`; **adopt mints `version_seq = 2`** and advances the current-version pointer (the model's second version producer — populate logic lands here, not scaffolding). / out — rename/move operators (M2).
  - **Relevant code**: `backend/crates/core/src/lib.rs` (verify + version mint + CAS pointer advance); `backend/crates/server/src/lib.rs` (endpoints)
  - **Added**: 2026-09-20
  - **Change**: _not yet proposed_

- [ ] 9. `bundled-postgres-install` — one command stands up Postgres beside the app
  - **Persona served**: Sam
  - **Journey segment**: Sam's Install
  - **MoSCoW**: Should
  - **Why this story / why now**: makes install effectively one command (ADR-004 packaging). The CI exit test only needs *a* Postgres, so this can trail the path.
  - **Depends on**: story 2
  - **Scope**: in — all-in-one image / compose bundle standing Postgres beside the app; forward-only migrations run on boot with downgrade guard. / out — production hardening, backups.
  - **Relevant code**: `backend/Dockerfile`; `.github/workflows/docker-build-*.yml`; a compose file
  - **Added**: 2026-09-20
  - **Change**: _not yet proposed_

- [ ] 10. `collection-lineage-api` — expose the scaffolding tables over minimal read endpoints
  - **Persona served**: Pat / future milestones
  - **Journey segment**: none in M1 (scaffolding surfacing)
  - **MoSCoW**: Could
  - **Why this story / why now**: the tables ship in story 2's migration, but nothing consumes them in M1. Only worth doing if a downstream (M3) design wants the read surface early. Otherwise leave for M3.
  - **Depends on**: story 2
  - **Scope**: in — minimal read API over `collection`(+member) and `lineage_edge`. / out — selectors, membership behavior, write paths (M2/M3).
  - **Relevant code**: `backend/crates/server/src/lib.rs`; `backend/crates/core/src/lib.rs`
  - **Added**: 2026-09-20
  - **Change**: _not yet proposed_

## Open Questions

- **ADR-011 (Proposed) gates all storage/ingest/on-disk work.** Per the STOP-on-open-ADR rule the storage tree, ingest protocol, and GC are HELD until ADR-011 is Accepted. This is a decision gate, not a build story — how do we sequence it ahead of the storage slices? (Issue lists it as step 0 / prerequisite.)
- **Adopt mints a second version.** _Resolved into story 8:_ `integrity/resolve {adopt}` is M1's second version producer, so version-populate logic (mint `version_seq = 2` + CAS pointer advance) lands there, not as scaffolding-only. Mike's suggested wording is correct.
- **Orphan in `store/` after a crash between `rename` and DB commit.** _Folded into story 2 scope:_ the startup orphan sweep must cover `store/` (objects with no referencing row), not only `tmp/`. Confirm the ADR-011 ingest protocol commits the row before the rename is considered durable, or sweeps unreferenced objects.
- **Edit-in-place changes `external_id`.** _Accepted as an M1 limitation (noted on story 5):_ a watched file edited in place changes size/mtime, so `external_id` (path+size+mtime) no longer matches and it ingests as a new asset, not a new version. Revisit when the Source manifest/config work lands in M2.
- **Testing gaps.** Mike flagged the test list as incomplete; revisit once stories are cut.

## Change Log

- 2026-09-18 — Initial plan from issue #217. Phase 1 (ingest) captured: scope restated, goals/non-goals lifted from the issue, open questions surfaced from Mike's review comment.
- 2026-09-20 — Phases 2–3 confirmed: three personas (Pat/Nova/Sam) and their journey maps, annotated against the M0 backend.
- 2026-09-20 — Phases 4–6 confirmed and plan finalized. MoSCoW set (integrity reconciliation moved Must→Should per product owner). Issue #217's horizontal steps 0–10 re-cut into 10 vertical stories: skeleton (story 2) proves the spine, story 3 is the contract-meets-reality slice, kept whole. Three of Mike's open questions resolved into story scope (adopt→story 8, store-orphan sweep→story 2, edit-in-place→story 5 limitation).
