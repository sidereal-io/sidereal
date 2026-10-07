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

Stage status checked against the code on 2026-10-06. Status describes the v2 backend
in `server/`; where v0.10.x already covers a stage, the stage says so.

### People who use Sidereal

**Library journey** (Nova):

```
  Connect source → Ingest safely → Read metadata → View images → Describe → Find
       gap             gap             gap             gap          gap       gap
                                                                              │
  Publish ← Trace lineage ← Match calibration ← Group sessions ←───────────────┘
     gap          gap               gap                gap
```

1. **Connect a source** — point Sidereal at a local folder or configure another
   Source plugin, such as Immich — gap. The Source reports candidates; core owns
   ingestion.
2. **Ingest safely** — copy files into managed storage, calculate BLAKE3 hashes,
   preserve source identity, and record immutable versions — gap. Identical content
   may share stored bytes without making the hash the logical asset identity.
3. **Read metadata** — inspect file properties and astronomy headers through
   schema-defined facets, with units and producer provenance — gap. Native FITS
   and XISF readers are absent, as are generic metadata Operators.
4. **View images** — browse image previews, open an individual image, zoom and pan,
   and inspect its metadata — gap. `web/src/App.tsx` only shows server health.
   v0.10.x has a gallery and OpenSeadragon viewer in `apps/client/src/`.
5. **Describe** — add tags and additional metadata; edit names, descriptions, and
   associations — gap. User descriptions and corrections remain distinguishable
   from extracted observations. v0.10.x has image editors; its tag badges do not
   establish a complete tag-editing workflow.
6. **Find** — search and combine criteria such as target, date, frame type, filter,
   equipment, tags, and processing state — gap. v0.10.x has a mixture of server
   filters and client-side search, date, and integration filters.
7. **Group sessions** — associate frames with sessions, targets, equipment,
   locations, and acquisitions; inspect integration totals — gap. v0.10.x has
   acquisition entries and equipment relations, but no raw-frame session model.
8. **Match calibration** — find compatible darks, flats, and masters by their
   recorded properties — gap.
9. **Trace lineage** — inspect which exact versions and calibration inputs produced
   a derived image — gap. Sidereal records or orchestrates external processing;
   it does not perform calibration or integration mathematics.
10. **Publish** — choose assets and publish through a Sink plugin, such as Immich,
    Astrobin, or a static gallery — gap. Removing a remote asset does not delete
    the managed Sidereal original.

> **Implication**: Raw frames, derived images, and finished images share an asset
> library. Viewing, descriptions, and retrieval are explicit parts of that path.
> An Immich integration can provide both Source and Sink capabilities through the
> same public plugin contracts as other integrations.

**Planning and sky journey** (Nova):

```
  Set up gear & sites → Explore targets → Check visibility → Frame a shot
           gap                 gap              gap               gap
                                                                    │
  Inspect annotations ← Explore captured images on a sky map ← Plate solve
           gap                         gap                        gap
```

1. **Set up gear and sites** — maintain equipment, equipment groups, and named
   observing locations — gap in v2. v0.10.x implements these in the equipment and
   locations pages and their server routes.
2. **Explore targets** — browse and search the catalog, record notes and tags, and
   see targets already imaged — gap in v2. v0.10.x has the Targets page.
3. **Check visibility** — evaluate a target for a site and observing time — gap in
   v2. v0.10.x has location-based visibility and target ordering.
4. **Frame a shot** — inspect the telescope/camera field of view on an interactive
   sky atlas — gap in v2. v0.10.x calculates this from equipment specifications in
   `apps/client/src/pages/sky-map.tsx`.
5. **Plate solve** — request a solution for one image or a selected batch and
   inspect progress, failures, and results — gap in v2. v0.10.x submits images to
   Astrometry.net.
6. **Explore captured images on a sky map** — pan and zoom an interactive atlas,
   see images at their sky positions, and open them in the viewer — gap in v2.
   v0.10.x has an Aladin-based Sky Map page. A usable sky position or plate solution
   is needed; an unsolved image remains accessible in the library.
7. **Inspect annotations** — view object labels and plate-solution details on an
   image — gap in v2. v0.10.x overlays annotations in its image viewer.

> **Implication**: Planning and reviewing captured sky coverage are two uses of
> the same equipment, location, catalog, and coordinate information. A person can
> enter at the library or the sky map; a fresh capture need not have a saved plan.

**Self-host journey** (Sam):

```
  Install → Establish private access → Configure → Protect originals
  partial              gap                gap               gap
                                                              │
  Upgrade from v0.10.x ← Back up & restore ← Diagnose & repair ←┘
          gap                   gap                  gap
```

1. **Install** — Postgres and the app come up together from one command — partial:
   `server/Dockerfile` and the axum shell exist, with no database or compose bundle.
2. **Establish private access** — set an administrator credential, sign in, and
   revoke access, with protected HTTP and WebSocket sessions — gap. The current
   server router serves only `/healthz`; there is no authentication flow.
3. **Configure** — choose storage, Sources, and processing/publishing integrations
   and test their connections — gap. v0.10.x has administration forms for Immich
   and Astrometry.net.
4. **Protect originals** — ingestion preserves watched files and limits plugins to
   approved capabilities; inspect processing state and failures — gap.
5. **Diagnose and repair** — find missing or corrupt objects, inspect their
   provenance, and deliberately adopt, restore, or ignore a mismatch — gap.
6. **Back up and restore** — verify a backup of both PostgreSQL and asset storage
   and restore a usable library — gap.
7. **Upgrade from v0.10.x** — run a read-only, resumable importer with a dry run,
   checksum verification, and a report of records or originals that did not map —
   gap.

**Plugin journey** (Pat):

```
  Code to contract → Declare capabilities → Register/install → Run
       partial               gap                partial        gap
                                                                │
  Use a published contract ← Pass conformance ← Inspect outcomes ←┘
             gap                    gap                  gap
```

1. **Code to the contract** — implement Source, Operator, or Sink behavior against
   `plugin-abi` — partial: traits in `server/crates/plugin-abi/src/lib.rs` carry
   only an id. There is no executable `AssetContext` or script/provider adapter.
2. **Declare capabilities** — declare schemas, configuration, outcomes, and
   requested byte, facet, network, or secret access — gap.
3. **Register or install** — contribute a built-in pack or install an extension
   with explicit approval of its grants — partial. `Pack::register` and `Registry`
   work for compiled components, but there is no extension loader or approval flow.
4. **Run** — core selects eligible work, supplies mediated capabilities, validates
   results, and commits outcomes — gap. There is no executor, goal state, or
   reconciler in `server/crates/core/src/lib.rs`.
5. **Inspect outcomes** — see results, provenance, progress, failures, and
   cancellation, without ambient access to managed bytes or secrets — gap.
6. **Pass conformance** — prove authorization, schema compatibility, original
   safety, and declared side-effect and recovery behavior — gap. The registration
   test checks ids only.
7. **Use a released contract** — build against a documented, versioned capability
   contract and know its compatibility limits — gap. The ABI remains an unfinished
   scaffolding contract. Preview contracts may evolve; a stable designation needs
   evidence from real consumers and conformance checks.

### People who build Sidereal

**Contributor journey** (Ada, Nico, Dana, Mo):

```
  Clone ─► Enter env ─► Get skills ─► Work & check ─► Open PR ─► Bump pins (Mo)
    │          │             │              │             │            │
 supported  supported     partial        partial       partial     supported
```

1. **Clone** — `git clone` works — supported.
2. **Enter env** — `flake.nix`, `nix/`, and `.envrc` give Nico a pinned shell through
   direnv; Dana installs the same tools by hand — supported.
3. **Get skills** — `just skills` generates the same skills on every machine, and the
   shell refreshes them when it loads. An agent started outside the shell can still
   run a different `openspec` from the one that generated its skills. The project
   accepts this gap and does not plan to close it — partial.
4. **Work & check** — `just dev` runs the v2 server and web shell together, and
   `just check` gates both the Rust workspace and `web/` with pinned tools.
   `DESIGN.md` defines the v2 design system, but the web shell doesn't use its
   tokens yet, has no components to build screens from, and has no browser smoke
   test — partial ([#302](https://github.com/sidereal-io/sidereal/issues/302),
   [#337](https://github.com/sidereal-io/sidereal/issues/337),
   [#303](https://github.com/sidereal-io/sidereal/issues/303)).
5. **Open PR** — CI checks the v2 server and web code inside the pinned shell, but
   runs no end-to-end browser check — partial
   ([#303](https://github.com/sidereal-io/sidereal/issues/303)).
6. **Bump pins** — Dependabot opens weekly pull requests for npm (root and `web/`),
   Docker, Actions, `flake.lock` and `server/Cargo.lock`. Rust follows the stable
   channel, so a new Rust release arrives in the `flake.lock` update, and the flake
   check prints its version — supported.

## Backlog

Stories are GitHub issues: https://github.com/sidereal-io/sidereal/issues.
