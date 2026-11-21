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

- **Who**: runs Sidereal on their own hardware, such as a home server or NAS, usually
  behind a reverse proxy they also run, and may be upgrading an existing install.
- **Goal**: stand Sidereal up quickly, keep it running, and trust it with files that
  cannot be replaced.
- **Pain today**: photo tools either want files in someone else's cloud or take over
  the folder layout. One bug can corrupt originals that took months of clear nights to
  capture, and a major-version upgrade puts the whole library at risk. A published port
  is reachable by everyone on the network, and a lost password too often means a lost
  install.
- **Success looks like**: installs with one command; only Sam can get in, and Sam can
  always get back in from the box itself; originals are never altered; corruption is
  detected and can be repaired; upgrades run against a verified backup and report
  anything that didn't carry over.

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

#### Nico — the contributor with a tool manager

- **Who**: a contributor who lets a tool manager install a project's pinned tools.
- **Goal**: go from clone to a passing check with no manual installs.
- **Pain today**: a Rust-and-Node repo usually means a page of install steps and
  versions that drift from what CI uses.
- **Success looks like**: one trust-and-install step after cloning is the whole
  setup, and the tools turn on whenever they enter the repo.

#### Dana — the contributor who installs tools by hand

- **Who**: a contributor who installs tools by hand and doesn't want to adopt a
  project's tool manager.
- **Goal**: contribute without adopting the project's tool manager.
- **Pain today**: projects that adopt a tool manager often leave everyone else on an
  unsupported, second-class path.
- **Success looks like**: with tools installed by hand, gets the same checks and the
  same skills as everyone else.

#### Mo — the maintainer

- **Who**: reviews and merges pull requests and keeps the toolchain current.
- **Goal**: trust that every pull request is built with the pinned tools and commits
  no generated drift, and keep the pins fresh without toil.
- **Pain today**: generated files and tool pins drift quietly until something breaks.
  Keeping the development environment working should not need knowledge of a
  specialist language that few maintainers have.
- **Success looks like**: CI fails on drift, and pin updates arrive as routine pull
  requests.

> **Implication**: Ada, Nico, Dana, and Mo walk one contributor path; they differ in
> how their tools arrive, not in what they do. Build one path that all of them reach,
> not one per setup.

## Journey Map

Stage status checked against the code on 2026-10-02; the self-host journey and the
library journey's sign-in stage re-checked on 2026-10-26; the contributor journey
re-checked on 2026-11-19; the library, self-host and plugin journeys re-checked on 2026-11-21.
Status describes the server
in `server/`; where v0.10.x already covers a stage, the stage says so.

### People who use Sidereal

**Library journey** (Nova):

```
  Sign in ─► Point at ─► Ingest   ─► Read     ─► See it   ─► Group    ─► Match    ─► Plate    ─► Trace    ─► Find &
             folder                  metadata                sessions    calibr.     solve       lineage     publish
     │          │           │           │           │           │           │           │           │           │
supported      gap         gap         gap         gap         gap         gap         gap         gap         gap
```

0. **Sign in** — Nova signs in to the private instance before anything else — supported.
1. **Point at a folder** — Sidereal watches a folder for new files — gap
   ([#358](https://github.com/sidereal-io/sidereal/issues/358)). Until then, Nova uploads files from the browser ([#521](https://github.com/sidereal-io/sidereal/issues/521), [#531](https://github.com/sidereal-io/sidereal/issues/531)).
2. **Ingest** — copy each file, hash it with BLAKE3, skip duplicates, and record an
   immutable version — gap: no schema and no store yet
   ([#521](https://github.com/sidereal-io/sidereal/issues/521), [#522](https://github.com/sidereal-io/sidereal/issues/522), [#523](https://github.com/sidereal-io/sidereal/issues/523), [#526](https://github.com/sidereal-io/sidereal/issues/526)).
3. **Read metadata** — size, format, and dimensions as facets, with the
   processing state visible — gap ([#524](https://github.com/sidereal-io/sidereal/issues/524), [#525](https://github.com/sidereal-io/sidereal/issues/525), [#527](https://github.com/sidereal-io/sidereal/issues/527)). FITS and XISF header reading has
   no story yet ([#357](https://github.com/sidereal-io/sidereal/issues/357)).
4. **See it** — the asset appears in the browser as it lands — gap: the web shell shows no assets yet
   ([#521](https://github.com/sidereal-io/sidereal/issues/521), [#526](https://github.com/sidereal-io/sidereal/issues/526), [#528](https://github.com/sidereal-io/sidereal/issues/528), [#530](https://github.com/sidereal-io/sidereal/issues/530)). The v0.10.x
   gallery shows Immich images only.
5. **Group into sessions** — frames sorted by type, target, filter, and equipment —
   gap.
6. **Match calibration** — a master dark or flat matched to the lights it fits by
   camera, temperature, gain, and exposure — gap.
7. **Plate solve** — gap on `main`. v0.10.x solves Immich images through Astrometry.net.
8. **Trace lineage** — which lights and masters produced a stack — gap
   ([#368](https://github.com/sidereal-io/sidereal/issues/368)). The first lineage link, a preview to its source, comes with [#526](https://github.com/sidereal-io/sidereal/issues/526).
9. **Find & publish** — search by target, filter, or equipment, and publish to Immich,
   Astrobin, or a static gallery — gap on `main`. v0.10.x browses and filters Immich
   images.

**Self-host journey** (Sam):

```
  Install ─► Configure ─► Claim ─► Sign in ─► Recover ─► Protect  ─► Detect & ─► Back up & ─► Upgrade from
  (1 cmd)                 admin     & out      access     originals   repair      restore      v0.10.x
     │          │           │          │          │           │           │           │            │
 supported  supported  supported  supported  supported     gap         gap         gap          gap
```

1. **Install** — `compose.yml` and `.env` bring up the app and PostgreSQL together, and
   `docs/install.md` walks through it — supported.
2. **Configure** — storage root, database and bind mounts through `.env`, and an
   existing PostgreSQL service through a Compose override — supported. Watched folders
   are a gap owned by [#358](https://github.com/sidereal-io/sidereal/issues/358).
3. **Claim administrator** — the first visitor to an unclaimed instance creates the
   sole administrator, or the operator does it from the box with `siderealctl` — supported.
4. **Sign in and out** — a session in a secure cookie, revoked on sign-out or expiry;
   every other caller is refused — supported.
5. **Recover access** — reset a lost administrator password from the box, signing out
   every session — supported.
6. **Protect originals** — ingest copies and never moves, renames, or deletes files in
   the watched folder — gap ([#521](https://github.com/sidereal-io/sidereal/issues/521), [#523](https://github.com/sidereal-io/sidereal/issues/523)).
7. **Detect & repair** — a corrupted stored file is found, and the user can adopt,
   restore, or ignore it — gap ([#371](https://github.com/sidereal-io/sidereal/issues/371)).
8. **Back up & restore** — a documented, verified backup of both the database and the
   storage root — gap ([#371](https://github.com/sidereal-io/sidereal/issues/371)).
9. **Upgrade from v0.10.x** — a one-way importer with a dry run and a report of what
   didn't map — gap ([#372](https://github.com/sidereal-io/sidereal/issues/372)).

**Plugin journey** (Pat):

```
  Code to    ─► Read bytes ─► Declare    ─► Register   ─► Run via    ─► Back door  ─► Pass
  contract      & facets      grants                      executor      refused       conform.
      │             │             │             │             │             │             │
   partial         gap           gap        supported        gap           gap           gap
```

1. **Code to the contract** — implement a Source or Operator against `plugin-abi` —
   partial: the traits in `server/crates/plugin-abi/src/lib.rs` carry only an id
   ([#521](https://github.com/sidereal-io/sidereal/issues/521), [#524](https://github.com/sidereal-io/sidereal/issues/524)). The embedded-script
   profile has no story yet.
2. **Read bytes & emit facets** — through `AssetContext`: byte access, `emit_facet`,
   `log`, `is_cancelled` — gap ([#524](https://github.com/sidereal-io/sidereal/issues/524)).
3. **Declare grants** — a manifest names the facets the plugin may write — gap
   ([#524](https://github.com/sidereal-io/sidereal/issues/524)).
4. **Register** — `Pack::register` adds the plugin to the `Registry` — supported.
5. **Run via the executor** — the executor dispatches the plugin and validates its
   outcome — gap ([#524](https://github.com/sidereal-io/sidereal/issues/524), [#527](https://github.com/sidereal-io/sidereal/issues/527)).
6. **Back door refused** — a direct store write or forged facet is rejected — gap
   ([#524](https://github.com/sidereal-io/sidereal/issues/524); proven in CI by [#529](https://github.com/sidereal-io/sidereal/issues/529)).
7. **Pass conformance** — the plugin passes the shared conformance suite — gap.

### People who build Sidereal

**Contributor journey** (Ada, Nico, Dana, Mo):

```
  Clone ─► Enter env ─► Get skills ─► Work & check ─► Open PR ─► Keep tools current (Mo)
    │          │             │              │             │                 │
 supported  supported     partial       supported     supported          partial
```

1. **Clone** — `git clone` works — supported.
2. **Enter env** — `flake.nix`, `nix/`, and `.envrc` give Nico a pinned shell through
   direnv; Dana installs the same tools by hand, from `server/rust-toolchain.toml`
   and `.nvmrc` — supported.
3. **Get skills** — `lets sync` copies the same skills, schemas and rules into
   every clone when the shell loads, and they are committed, so an agent started
   outside the shell reads the same text. The `openspec` it runs can still differ
   from the one the skills were generated from. The project accepts this gap and
   does not plan to close it — partial.
4. **Work & check** — `mise run dev` runs the server and web shell together, and
   `mise run check` gates the Compose files, OpenSpec, the Rust workspace and `web/`
   with pinned tools, including the token drift check — supported.
5. **Open PR** — CI runs the same checks inside the pinned shell, plus an
   end-to-end browser check — supported.
6. **Keep tools current (Mo)** — Dependabot opens weekly pull requests for npm
   (root and `web/`), Docker, Actions, `flake.lock` and `server/Cargo.lock`. Rust
   follows the stable channel, so a new Rust release arrives in the `flake.lock`
   update. A `flake.lock` bump that moves Let's Spec fails `lets sync --check`
   until someone enters the shell and commits what `lets sync` wrote, and changing
   the shell itself needs Nix and flake-parts — partial
   ([#501](https://github.com/sidereal-io/sidereal/issues/501),
   [#502](https://github.com/sidereal-io/sidereal/issues/502)).

## Backlog

Stories are GitHub issues: https://github.com/sidereal-io/sidereal/issues.
