# Discovery

The living map behind the backlog: who it serves and the journeys it supports.

The product input is GitHub issue [#217](https://github.com/sidereal-io/sidereal/issues/217)
"M1: Core spine & first plugins", with the [migration plan](migration.md),
for M1; and [ADR-013](../docs/decisions/ADR-013-development-environment.md) with the epic
[#272](https://github.com/sidereal-io/sidereal/issues/272) for the dev environment. Stories are GitHub
issues: each issue body is the story packet, with its MoSCoW priority.

## Personas

### Dev environment

- **Ada, the AI coding agent** (Claude, Codex, Gemini, and others). *Goal:* start
  every session with the exact tools and OpenSpec skills the repo expects. *Pain
  today:* skill text depends on whoever's machine it is, and the committed skills
  contain Codex-specific hints. *Success:* the same `openspec --version`, the same
  toolchain versions, and the same skills in every session, with no setup steps to
  work out.
- **Nico, the contributor with Nix.** *Goal:* go from clone to `just check` with no
  manual installs. *Success:* running `direnv allow` once is the whole setup.
- **Dana, the contributor without Nix.** *Goal:* contribute without learning Nix.
  *Pain today:* no clear, supported way to get the OpenSpec skills. *Success:*
  `just skills` gives the same skills using any `openspec` install.
- **Mo, the maintainer.** *Goal:* trust that pull requests don't commit generated
  files and that the pinned CLI regenerates cleanly. *Pain today:* generated skills
  are committed and nothing checks them. *Success:* a CI check fails on drift.

### M1

- **Pat, the plugin author.** Builds a Source or Operator against `plugin-abi`. In M1
  this is a Sidereal maintainer *wearing the third-party hat*: writing the two
  built-ins with no privileges a real third party wouldn't have. *Goal:* make a
  file-discovering Source and a metadata-emitting Operator work end to end through
  the public contract alone. *Pain today:* `plugin-abi` is id-stubs: no
  `AssetContext`, no byte access, no facet emission, and no proof the contract is
  expressive enough for real work. *Success:* both plugins run through the same
  surface a stranger would use, and any attempt to reach around it (direct store
  write, forged facet) is rejected.
- **Nova, the astrophotographer.** The end user who will eventually manage thousands
  of frames. *Goal:* drop a file into a folder and see Sidereal recognize it. *Pain
  today:* v2 turns nothing into a managed asset; the backend only serves `/healthz`.
  *Success:* a file appears in the UI on its own, with size, format and dimensions
  filled in, within seconds of landing in the folder. *Honest scope:* M1 is a sliver
  of Nova's journey (generic metadata only, read-only UI, no astro). Her real payoff
  is M3.
- **Sam, the self-hoster.** Runs the Sidereal box on their own hardware. *Goal:*
  stand it up in roughly one command and trust it with their files. *Pain today:*
  nothing to run, no storage story, no assurance originals are safe. *Success:*
  Postgres and the app come up bundled; originals in the watched folder are never
  moved or altered; and if a stored file is corrupted, Sidereal detects it and offers
  a fix (adopt, restore or ignore).

## Journey Map

### Dev environment

```
  Clone --> Enter env --> Get skills --> Work & check --> Open PR --> Bump pins
    |           |             |               |               |            |
 supported  supported      partial        supported          gap        partial
```

1. **Clone.** `git clone` works. *Supported.*
2. **Enter env.** `flake.nix`, `nix/` and `.envrc` give a pinned shell through
   direnv. *Supported* ([#273](https://github.com/sidereal-io/sidereal/issues/273)).
3. **Get skills.** The skills are committed, but they are Codex-flavored, depend on
   the global OpenSpec config, and aren't regenerated. *Partial*
   ([#274](https://github.com/sidereal-io/sidereal/issues/274),
   [#275](https://github.com/sidereal-io/sidereal/issues/275)).
4. **Work & check.** `just check` runs with pinned Rust, Node, `just` and `openspec`.
   *Supported* ([#273](https://github.com/sidereal-io/sidereal/issues/273)).
5. **Open PR.** CI checks the code, but nothing checks the skills. *Gap*
   ([#276](https://github.com/sidereal-io/sidereal/issues/276)).
6. **Bump pins.** `flake.lock` exists, but there is no update routine. *Partial*
   ([#287](https://github.com/sidereal-io/sidereal/issues/287)).

### M1: Nova's spine, the asset path

This path is M1's exit criterion. M0's one solid rung is the registration path
(`Pack::register` + `Registry`). The storage layout is decided
([ADR-011](../docs/decisions/ADR-011-storage-tree-layout.md), Accepted).

```
  Run app ──► Point at ──► Drop ──► Ingest ──► Extract ──► See in ──► Live
  (Postgres)   folder      file    (hash/       metadata    UI        update
                                    store/ver)                         (WS)
     │           │           │        │            │          │          │
   partial     gap         gap      gap          gap        gap        gap
```

1. **Run app.** Bundled Postgres and app boot. *Partial:* `backend/Dockerfile` and
   the axum shell exist; no database, no compose bundle
   ([#285](https://github.com/sidereal-io/sidereal/issues/285)).
2. **Point at a watched folder.** Config or env. *Gap*
   ([#281](https://github.com/sidereal-io/sidereal/issues/281)).
3. **Drop a file, auto-detected.** Source, `notify` and debounce. *Gap*
   ([#281](https://github.com/sidereal-io/sidereal/issues/281)).
4. **Ingest.** Copy, BLAKE3 hash, dedup, store, mint `AssetVersion`. *Gap:* no schema,
   no store ([#278](https://github.com/sidereal-io/sidereal/issues/278)).
5. **Extract metadata.** The executor runs the Pure operator and emits `core.*`
   facets. *Gap* ([#279](https://github.com/sidereal-io/sidereal/issues/279),
   [#280](https://github.com/sidereal-io/sidereal/issues/280)).
6. **See in UI.** Read-only asset list and detail. *Gap:* the frontend in `apps/client`
   is v0.10.x; there is no v2 shell
   ([#282](https://github.com/sidereal-io/sidereal/issues/282)).
7. **Live update.** `asset.ingested` over `/ws`. *Gap*
   ([#279](https://github.com/sidereal-io/sidereal/issues/279),
   [#282](https://github.com/sidereal-io/sidereal/issues/282)).

### M1: Pat's overlay, build a plugin through the contract

```
  Code to ──► Get ──► Read bytes ──► Emit ──► Register ──► Run via ──► Back-door
  contract   AssetCtx   /facets    facet     plugin      executor    rejected
     │          │          │          │         │            │           │
  partial     gap        gap        gap    supported       gap         gap
```

- **Code to contract.** *Partial:* the `Source` and `Operator` traits in
  `backend/crates/plugin-abi/src/lib.rs` are id-only
  ([#279](https://github.com/sidereal-io/sidereal/issues/279),
  [#281](https://github.com/sidereal-io/sidereal/issues/281)).
- **AssetContext** (byte read, `emit_facet`, `log`, `is_cancelled`). *Gap*
  ([#279](https://github.com/sidereal-io/sidereal/issues/279)).
- **Register plugin.** *Supported:* `Pack::register` + `Registry`, proven in M0.
- **Run via executor, outcome validated, no back door.** *Gap*
  ([#279](https://github.com/sidereal-io/sidereal/issues/279); proven in CI by
  [#283](https://github.com/sidereal-io/sidereal/issues/283)).

### M1: Sam's overlay, install and trust

```
  Install ──► Configure ──► Originals ──► Detect ──► Repair
  (1 cmd)                   untouched    corruption  (adopt/restore/ignore)
     │           │             │            │            │
  partial       gap           gap          gap          gap
```

- **One-command install.** *Partial:* `backend/Dockerfile` exists; no bundled-Postgres
  compose ([#285](https://github.com/sidereal-io/sidereal/issues/285)).
- **Configure.** *Gap* ([#281](https://github.com/sidereal-io/sidereal/issues/281)).
- **Filesystem safety** (copy, never move). *Gap*
  ([#278](https://github.com/sidereal-io/sidereal/issues/278)).
- **Integrity detect and repair.** *Gap*
  ([#284](https://github.com/sidereal-io/sidereal/issues/284)).

## Backlog

Stories are GitHub issues: https://github.com/sidereal-io/sidereal/issues.
