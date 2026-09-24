# Discovery: M1 — Core spine & first plugins

> Status: complete
> Created: 2026-09-18 · Last revised: 2026-09-24

> Release plan produced by the discovery skill. Resume or revise by re-running the skill.
> To build: run `/opsx:propose` and ask it to use the next unchecked story below.
> One story = one OpenSpec change (proposal ≈ 200 words). Create one at a time.

> **Two tracks, in build order.** Track E (dev environment, stories E1–E4) comes
> first and is independent of M1. The M1 plan follows it, starting at
> [Sources](#sources); its story numbers are unchanged.

## Track E — Reproducible, agent-native dev environment

### What this track delivers

One reproducible development environment for this repo. It pins the Rust and Node
toolchains and the `openspec` CLI, and it turns on automatically when you enter the
repo, through direnv. Every agent (Claude, Codex, Gemini and others) and every
contributor gets the same tools and the same OpenSpec skills, without having to
work anything out.

**Decisions already made** (in an explore session on 2026-09-23; don't re-open them):

- **Native Nix flakes with flake-parts, not devenv.** The deciding factors:
  - no wrapper CLI for contributors or CI to install;
  - `nix flake check` gives CI a native place for checks;
  - flake-parts modules can later be extracted into a shared, vendor-free flake.
  - The accepted cost: services such as Postgres will need services-flake later,
    which takes more setup than devenv's built-in services.
- **Nix is optional for humans.** `backend/rust-toolchain.toml` and `.nvmrc` remain
  the path for contributors without Nix. The environment guarantees the tools; it is
  not the only way in.
- **OpenSpec skills are generated, not committed.** They are generated only with
  `--tools agents,<list>`, where the list of extra agents starts as `claude`. There
  is no detection of which agents a contributor has installed: to add an agent,
  add one name to the list.
- **Authored skills live only in `.agents/skills/`.** Authored skill names must never
  start with `openspec-`, because that prefix is reserved for generated skills.
- **Excluded tools:** mise and devbox, because they aren't hermetic and composition
  is weaker; flox, because it depends on the hosted FloxHub service.

**What we found in the repo and CLI** (tested in scratch copies on 2026-09-23):

- **Generated skills depend on a per-user global config.** `openspec init` reads
  `~/.config/openspec/config.json` (profile, delivery, workflows). With a default
  config it generates 6 skills instead of 8, so the same pinned CLI can produce
  different skills on different machines. `XDG_CONFIG_HOME` redirects this config.
- **Only the `.agents/skills/` target and the listed agents' folders are written.**
  `openspec init --tools agents,claude` writes only `.agents/skills/openspec-*`,
  a `.agents/skills/.openspec-target` marker, and `.claude/skills/openspec-*`. It
  does not touch `AGENTS.md`. Running `openspec update` again leaves `git status`
  clean.
- **Per-agent outputs differ by one line only**, a "run this next" hint. With
  skills-only delivery, Claude's output is byte-identical to the generic output.
- **`openspec update` does not detect new agent folders.** It refreshes only the
  agents that are already set up.
- **Every `openspec update` checks the network for a newer CLI** and prints a notice.
- **`openspec` is not pinned today.** Version 1.12.0 is installed globally through
  npm. nixpkgs packages it (`pkgs/by-name/op/openspec`, 1.13.1 on master).
- **The committed skills were generated with `--tools codex`.**
  - That output contains Codex-specific hint text.
  - `.claude/skills` is a directory symlink to `.agents/skills`.
  - All eight generated `openspec-*` skills are committed.
- **The Node version is already inconsistent:** `.nvmrc` says `24.10.0`, CI uses
  `24.x`, and `AGENTS.md` says "Node 20+".

**Out of scope:** Postgres and service management (deferred to M1 story 9, when it
is needed), and the shared multi-repo "agent-conventions" flake. The environment is
split into modules so that later extraction stays possible.

### Personas

- **Ada, the AI coding agent** (Claude, Codex, Gemini, and others). *Goal:* start
  every session with the exact tools and OpenSpec skills the repo expects. *Pain
  today:* tool versions and skill text depend on whoever's machine it is, and the
  committed skills contain Codex-specific hints. *Success:* the same `openspec
  --version`, the same toolchain versions, and the same skills in every session,
  with no setup steps to work out.
- **Nico, the contributor with Nix.** *Goal:* go from clone to `just check` with no
  manual installs. *Pain today:* installs rustup, a Node version manager and a global
  `openspec` by hand, and the versions drift. *Success:* running `direnv allow` once
  is the whole setup.
- **Dana, the contributor without Nix.** *Goal:* contribute without learning Nix.
  *Pain today:* no clear, supported way to get the OpenSpec skills. *Success:*
  `just skills` gives the same skills using any `openspec` install.
- **Mo, the maintainer.** *Goal:* trust that pull requests don't commit generated
  files and that the pinned CLI regenerates cleanly. *Pain today:* generated skills
  are committed and nothing checks them. *Success:* a CI check fails on drift.

### Journey map

```
  Clone --> Enter env --> Get skills --> Work & check --> Open PR --> Bump pins
    |           |             |               |               |            |
 supported     gap         partial          partial          gap          gap
```

1. **Clone.** `git clone` works today. *Supported.*
2. **Enter env.** There is no flake and no `.envrc`; tools come from each
   contributor's own installs. *Gap.*
3. **Get skills.** The skills are committed, but they are Codex-flavored, depend on
   the global config, and aren't regenerated. *Partial.*
4. **Work & check.** `just check` exists, but its tool versions aren't pinned
   (`justfile`, `backend/rust-toolchain.toml` for Rust only). *Partial.*
5. **Open PR.** CI checks the code, but nothing checks the skills. *Gap.*
6. **Bump pins.** There is no lock file and no update routine for the environment.
   *Gap.*

### MoSCoW

**Must**

- A pinned dev shell that turns on automatically, for Ada and Nico (journey stages 2
  and 4).
- Reproducible skill generation through `just skills`: a repo-owned OpenSpec config,
  the fixed agent list, and authored skills linked into each listed agent's folder.
  This serves Ada and Dana (stage 3).
- Generated files are gitignored and removed from git, and the directory symlink is
  gone. Mo needs this so generated files stay out of commits (stage 3).
- Skills are generated when you enter the shell, with the update-check notice turned
  off. Ada and Nico get fresh skills without a manual step (stage 3).
- A CI check for skills drift, for Mo (stage 5).

**Should**

- Documentation in `AGENTS.md` and `CONTRIBUTING.md`: the Nix and non-Nix paths,
  the reserved `openspec-` prefix, and how to add an agent. Each story updates the
  docs it affects.

**Could**

- Scheduled `nix flake update` pull requests for Mo (stage 6).
- A Cachix binary cache, only once we build our own derivations. Everything in this
  plan is prebuilt on cache.nixos.org.
- Moving all CI jobs onto `nix develop`.

**Won't (this track)**

- Postgres or services; this belongs to M1 story 9.
- The shared agent-conventions flake; this track only keeps the modules separate.
- Detecting installed agents: generated files are gitignored and cheap, and
  detection would make output differ between machines.
- Requiring Nix: that would add friction for contributors who don't use it.
- devenv, mise, devbox and flox: see the decisions above.

### Stories

- [x] E1. `nix-dev-shell` — enter the repo and get pinned Rust, Node, `just` and `openspec` ⭐ walking skeleton
  - **Persona served**: Nico, Ada
  - **Journey segment**: Enter env and Work & check
  - **MoSCoW**: Must
  - **Why this story / why now**: this is the thinnest end-to-end path. After a clone,
    `direnv allow` gives a shell where `just check` passes with pinned tools. All
    other stories build on it.
  - **Depends on**: nothing
  - **Scope**:
    - In: a `flake.nix` using flake-parts, with separate modules under `nix/` (one
      for toolchains, one for openspec).
    - In: Rust from rust-overlay, reading `backend/rust-toolchain.toml` so that
      file stays the single source of truth.
    - In: `nodejs_26` and `just`.
    - In: `openspec` from pinned nixpkgs, or `overrideAttrs` for an exact version.
    - In: `.envrc` with `use flake` for nix-direnv, and `flake.lock`.
    - In: `AGENTS.md` changes from "Node 20+" to 26, and it describes the optional
      Nix path.
    - Out: skill generation (E2 and E3) and any CI change beyond keeping
      every workflow's Node version in sync with the pin (E4).
  - **Relevant code**: `justfile`; `backend/rust-toolchain.toml`; `.nvmrc`;
    `package.json`; `AGENTS.md`; `CONTRIBUTING.md`; `.gitignore` (add `.direnv/`);
    `.github/workflows/*.yml` (every file with a Node version); `Dockerfile`
  - **Added**: 2026-09-23
  - **Change**: `nix-dev-shell` (archived)

- [ ] E2. `generated-agent-skills` — `just skills` makes identical skills on every machine, and git holds only authored skills
  - **Persona served**: Dana, Ada, Mo
  - **Journey segment**: Get skills
  - **MoSCoW**: Must
  - **Why this story / why now**: this fixes the global-config leak and the Codex
    hint text. It also removes generated files from git. It works without Nix, so
    Dana is covered too.
  - **Depends on**: nothing (it can run in parallel with E1)
  - **Scope**:
    - In: a repo-owned OpenSpec config with 8 workflows and skills-only delivery.
      `just skills` points `XDG_CONFIG_HOME` at it and turns telemetry off.
    - In: `just skills` runs `openspec init --tools agents,<list>` (the list starts
      as `claude`) and links each authored skill into each listed agent's folder.
    - In: a `.gitignore` covering generated skills in every agent folder and the
      authored-skill links. Generated `openspec-*` skills are removed from git, and
      the `.claude/skills` directory symlink is replaced by a real folder.
    - In: `AGENTS.md` and `CONTRIBUTING.md` document `just skills`, the reserved
      prefix, and how to add an agent.
    - Out: running it automatically when entering the shell (E3).
  - **Relevant code**: `.agents/skills/*` (authored: `critique`, `grill-me`,
    `choose-an-adversary`); `.agents/skills/.openspec-target`; `.claude/skills`
    (symlink); `openspec/config.yaml`; `justfile`; `.gitignore`
  - **Added**: 2026-09-23
  - **Change**: _not yet proposed_

- [ ] E3. `skills-on-env-enter` — entering the shell refreshes the skills quietly and offline
  - **Persona served**: Ada, Nico
  - **Journey segment**: Enter env and Get skills
  - **MoSCoW**: Must
  - **Why this story / why now**: agents shouldn't need to know that they have to
    run `just skills`. This story joins E1 and E2 together.
  - **Depends on**: E1, E2
  - **Scope**:
    - In: a shell hook in the openspec flake module runs `just skills`, but only
      when the CLI version or the `openspec/` config changes (tracked by a stamp
      file).
    - In: the hook never blocks entering the shell.
    - In: turn off OpenSpec's update-check notice and network call in the
      environment. If no switch exists, use a timeout or report it upstream.
    - Out: CI (E4).
  - **Relevant code**: the openspec flake module from E1; `just skills` from E2
  - **Added**: 2026-09-23
  - **Change**: _not yet proposed_

- [ ] E4. `skills-drift-ci` — CI proves the pinned CLI regenerates the skills cleanly
  - **Persona served**: Mo
  - **Journey segment**: Open PR
  - **MoSCoW**: Must
  - **Why this story / why now**: without this check, generated files can slip back
    into commits and pins can drift unnoticed.
  - **Depends on**: E1, E2
  - **Scope**:
    - In: a check runs inside the Nix shell, exposed through `nix flake check` and
      run by a new CI job. It passes only when:
      - `just skills` on a clean checkout leaves `git status --porcelain` empty;
      - every generated `SKILL.md` has `generatedBy` equal to `openspec --version`;
      - `git ls-files` shows no tracked `openspec-*` skill.
    - In: Nix is installed in CI with the Determinate installer, with no Cachix.
    - Out: moving the existing `ci.yml` and `backend-rs.yml` jobs onto Nix
      (Could); scheduled flake updates (Could).
  - **Relevant code**: `.github/workflows/ci.yml`; `.github/workflows/backend-rs.yml`
    (the pattern to follow); the check module in the flake from E1
  - **Added**: 2026-09-23
  - **Change**: _not yet proposed_

### Open questions (Track E)

- **Which switch turns off OpenSpec's update check?** Not yet found. E3 depends on it.
- **How should `openspec` be pinned?** Follow the nixpkgs version (1.13.1 on master)
  or override to an exact version? This decides whether we ever build it ourselves,
  and so whether Cachix matters.
- **Which popular agents read `.agents/skills` natively?** Codex and Gemini do. We
  believe Copilot, Cursor and OpenCode do, but haven't verified it. The answer
  decides whether the agent list grows beyond `claude`.
- **Does each listed agent load a symlinked skill folder?** Check this once per agent.
- **What is the generic `.gitignore` rule** for authored-skill links in any listed
  agent's folder, so that adding an agent needs no `.gitignore` change?
- **Which platforms does the flake support?** Linux only, or macOS (`darwin`) too?
- **The maintainer's own setup:** global `openspec-*` and `opsx:*` skills in
  `~/.claude` duplicate the repo's skills. Remove them after E3 ships; we'll note
  this for contributors.

---

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
- 2026-09-23 — Revision: added Track E (reproducible, agent-native dev environment), which comes before M1 in build order. It came from an explore session that chose native Nix flakes with flake-parts over devenv, kept Nix optional, and made OpenSpec skills generated rather than committed (`--tools agents,claude`, no detection). Four stories, E1–E4, with E1 as the walking skeleton. M1 stories and numbering are unchanged, and no M1 story has changed state (no active or archived changes).
- 2026-09-24 — Revision: story E1's scope bullets updated from Node 24 to Node 26, to match the `nix-dev-shell` change (openspec/changes/nix-dev-shell/), which moved its target Node version after propose and review found E1's plan already implemented and approved. E1's "Out" bullet now also carves out keeping CI's `node-version` in sync with the pin, matching that change's design.md. Found and fixed by `/opsx:verify` catching the drift between this file and the change it seeded.
- 2026-09-24 — E1 (`nix-dev-shell`) archived after a 13-round review (10 rounds before archive; an external PR review then found 8 more findings — 6 real, 1 already fixed, 1 refuted with direct evidence — which reopened and re-closed the change through 3 more rounds) and 43/43 tasks verified. Delivered scope widened twice past what E1 originally scoped: every CI workflow's Node version, not just `ci.yml`'s, and the production `Dockerfile`. CI workflows now read `.nvmrc` through `node-version-file` rather than hardcoding the version, cutting the hand-synced locations from seven to three. `openspec/specs/dev-environment/spec.md` synced (11 requirements). E1 checked off; E2–E4 remain unstarted and unchanged. No M1 story's state changed.
