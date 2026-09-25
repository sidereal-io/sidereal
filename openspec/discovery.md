# Discovery

> The living map behind the backlog: who it serves and the journeys it supports.
> The backlog is GitHub issues. To build, run `/openspec-propose-change`; 
> it picks the next open, unblocked, unassigned issue.

## Personas

### People who use Sidereal

#### Nova — the astrophotographer

- **Who**: an amateur astrophotographer who shoots hundreds of light frames a session,
  plus darks, flats, and bias frames, and keeps the stacked and finished images too.
- **Goal**: keep every frame, from raw light to annotated final, organized and
  findable, along with the record of how each final image was made.
- **Pain today**: FITS and XISF files pile up across folders and drives. Nothing reads
  their headers, groups them into sessions, or remembers which master dark went into
  which stack. Answering "is this master dark still valid for this camera at this
  temperature?" means digging by hand. Gallery tools only understand finished photos.
- **Success looks like**: drops a night's files in a folder and finds them grouped by
  session and target, with calibration matched and lineage recorded; answers "which
  stacks used this master dark?" in seconds.

#### Sam — the self-hoster

- **Who**: runs Sidereal on their own hardware, such as a home server or NAS, and may
  be upgrading an existing install.
- **Goal**: stand Sidereal up quickly, keep it running, and trust it with files that
  cannot be replaced.
- **Pain today**: photo tools either want files in someone else's cloud or take over
  the folder layout. One bug can corrupt originals that took months of clear nights to
  capture, and a major-version upgrade puts the whole library at risk.
- **Success looks like**: installs with one command; originals are never altered;
  corruption is detected and can be repaired; upgrades run against a verified backup
  and report anything that didn't carry over.

#### Pat — the plugin author

- **Who**: a developer who extends Sidereal with a new file source, processing step,
  or publishing target. Pat may be a third party or a maintainer.
- **Goal**: build a plugin against a public, documented contract and have it behave
  like a built-in.
- **Pain today**: extending a gallery app usually means forking it or depending on
  internals that break on the next release.
- **Success looks like**: the plugin runs through the same contract the built-ins
  use, passes the conformance suite, and survives upgrades; anything outside the
  contract is refused, not quietly allowed.

### People who build Sidereal

#### Ada — the AI coding agent

- **Who**: Claude, Codex, Gemini, and other agents working in the repo.
- **Goal**: start every session with the tools and skills the repo expects.
- **Pain today**: an agent inherits whatever the host machine has. Tool versions and
  skill text vary between machines, and hints meant for one agent leak into another's.
- **Success looks like**: the same tool versions and the same skills in every
  session, with no setup steps to work out.

#### Nico — the contributor with Nix

- **Who**: a contributor who already uses Nix.
- **Goal**: go from clone to a passing check with no manual installs.
- **Pain today**: a Rust-and-Node repo usually means a page of install steps and
  versions that drift from what CI uses.
- **Success looks like**: running `direnv allow` once is the whole setup.

#### Dana — the contributor without Nix

- **Who**: a contributor who installs tools by hand and doesn't want to learn Nix.
- **Goal**: contribute without learning Nix.
- **Pain today**: projects that adopt Nix often leave everyone else on an unsupported,
  second-class path.
- **Success looks like**: with tools installed by hand, gets the same checks and the
  same skills as everyone else.

#### Mo — the maintainer

- **Who**: reviews and merges pull requests and keeps the toolchain current.
- **Goal**: trust that every pull request is built with the pinned tools and commits
  no generated drift, and keep the pins fresh without toil.
- **Pain today**: generated files and tool pins drift quietly until something breaks.
- **Success looks like**: CI fails on drift, and pin updates arrive as routine pull
  requests.

> **Implication**: Ada, Nico, Dana, and Mo walk one contributor path; they differ in
> how their tools arrive, not in what they do. Build one path that all of them reach,
> not one per setup.

## Journey Map

Stage status checked against the code on 2026-09-25. Status describes the v2 backend
in `backend/`; where v0.10.x already covers a stage, the stage says so.

### People who use Sidereal

**Library journey** (Nova):

```
  Point at ─► Ingest   ─► Read     ─► See it   ─► Group    ─► Match    ─► Plate    ─► Trace    ─► Find &
  folder                  metadata                sessions    calibr.     solve       lineage     publish
     │           │           │           │           │           │           │           │           │
    gap         gap         gap         gap         gap         gap         gap         gap         gap
```

1. **Point at a folder** — Sidereal watches a folder for new files — gap
   ([#281](https://github.com/sidereal-io/sidereal/issues/281)).
2. **Ingest** — copy each file, hash it with BLAKE3, skip duplicates, and record an
   immutable version — gap: no schema and no store yet
   ([#278](https://github.com/sidereal-io/sidereal/issues/278)).
3. **Read metadata** — size, format, and dimensions as facets — gap
   ([#279](https://github.com/sidereal-io/sidereal/issues/279),
   [#280](https://github.com/sidereal-io/sidereal/issues/280)). FITS and XISF header
   reading has no story yet.
4. **See it** — the asset appears in the browser as it lands — gap: no v2 frontend
   shell ([#282](https://github.com/sidereal-io/sidereal/issues/282)). The v0.10.x
   gallery shows Immich images only.
5. **Group into sessions** — frames sorted by type, target, filter, and equipment —
   gap.
6. **Match calibration** — a master dark or flat matched to the lights it fits by
   camera, temperature, gain, and exposure — gap.
7. **Plate solve** — gap in v2. v0.10.x solves Immich images through Astrometry.net.
8. **Trace lineage** — which lights and masters produced a stack — gap
   ([#286](https://github.com/sidereal-io/sidereal/issues/286) exposes the scaffolding
   tables).
9. **Find & publish** — search by target, filter, or equipment, and publish to Immich,
   Astrobin, or a static gallery — gap in v2. v0.10.x browses and filters Immich
   images.

**Self-host journey** (Sam):

```
  Install ─► Configure ─► Protect ─► Detect & ─► Back up & ─► Upgrade from
  (1 cmd)                 originals   repair      restore      v0.10.x
     │           │            │           │           │            │
  partial       gap          gap         gap         gap          gap
```

1. **Install** — Postgres and the app come up together from one command — partial:
   `backend/Dockerfile` and the axum shell exist, with no database or compose bundle
   ([#285](https://github.com/sidereal-io/sidereal/issues/285)).
2. **Configure** — storage root and watched folders set through config or environment
   — gap ([#281](https://github.com/sidereal-io/sidereal/issues/281)). An admin
   configuration UI has no story yet.
3. **Protect originals** — ingest copies and never moves, renames, or deletes files in
   the watched folder — gap ([#278](https://github.com/sidereal-io/sidereal/issues/278)).
4. **Detect & repair** — a corrupted stored file is found, and the user can adopt,
   restore, or ignore it — gap ([#284](https://github.com/sidereal-io/sidereal/issues/284)).
5. **Back up & restore** — a documented, verified backup of both the database and the
   storage root — gap.
6. **Upgrade from v0.10.x** — a one-way importer with a dry run and a report of what
   didn't map — gap.

**Plugin journey** (Pat):

```
  Code to    ─► Read bytes ─► Declare    ─► Register   ─► Run via    ─► Back door  ─► Pass
  contract      & facets      grants                      executor      refused       conform.
      │             │             │             │             │             │             │
   partial         gap           gap        supported        gap           gap           gap
```

1. **Code to the contract** — implement a Source or Operator against `plugin-abi` —
   partial: the traits in `backend/crates/plugin-abi/src/lib.rs` carry only an id
   ([#279](https://github.com/sidereal-io/sidereal/issues/279),
   [#281](https://github.com/sidereal-io/sidereal/issues/281)). The embedded-script
   profile has no story yet.
2. **Read bytes & emit facets** — through `AssetContext`: byte access, `emit_facet`,
   `log`, `is_cancelled` — gap ([#279](https://github.com/sidereal-io/sidereal/issues/279)).
3. **Declare grants** — a manifest names the facets the plugin may write — gap.
4. **Register** — `Pack::register` adds the plugin to the `Registry` — supported.
5. **Run via the executor** — the executor dispatches the plugin and validates its
   outcome — gap ([#279](https://github.com/sidereal-io/sidereal/issues/279)).
6. **Back door refused** — a direct store write or forged facet is rejected — gap
   ([#279](https://github.com/sidereal-io/sidereal/issues/279); proven in CI by
   [#283](https://github.com/sidereal-io/sidereal/issues/283)).
7. **Pass conformance** — the plugin passes the shared conformance suite — gap.

### People who build Sidereal

**Contributor journey** (Ada, Nico, Dana, Mo):

```
  Clone ─► Enter env ─► Get skills ─► Work & check ─► Open PR ─► Bump pins (Mo)
    │          │             │              │             │            │
 supported  supported     partial       supported      partial      partial
```

1. **Clone** — `git clone` works — supported.
2. **Enter env** — `flake.nix`, `nix/`, and `.envrc` give Nico a pinned shell through
   direnv; Dana installs the same tools by hand — supported.
3. **Get skills** — skills are committed, but they carry Codex-specific hints, depend
   on the global OpenSpec config, and aren't regenerated — partial
   ([#274](https://github.com/sidereal-io/sidereal/issues/274),
   [#275](https://github.com/sidereal-io/sidereal/issues/275)).
4. **Work & check** — `just check` runs with pinned Rust, Node, `just`, and
   `openspec` — supported.
5. **Open PR** — CI checks the code, but not the skills, and not inside the pinned
   shell — partial ([#276](https://github.com/sidereal-io/sidereal/issues/276),
   [#289](https://github.com/sidereal-io/sidereal/issues/289)).
6. **Bump pins** — `flake.lock` exists, but nothing updates it on a schedule — partial
   ([#287](https://github.com/sidereal-io/sidereal/issues/287)).

## Backlog

Stories are GitHub issues: https://github.com/sidereal-io/sidereal/issues.
