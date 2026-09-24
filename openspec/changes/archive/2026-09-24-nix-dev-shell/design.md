## Context

See `proposal.md` (Why) for the motivation and `specs/dev-environment/spec.md` for the required behavior. This section covers only the facts that shape the approach.

- **Rust is already pinned** in `backend/rust-toolchain.toml` at channel `1.85.0`, with `rustfmt` and `clippy`. rustup reads this file today.
- **`just check` needs the Rust toolchain and a few standard Unix tools.** `backend/scripts/check-arch.sh` calls `bash`, `cargo tree`, `grep` and `sed`. The Nix shell supplies the Unix tools through nixpkgs' standard environment, which includes `coreutils`, `gnugrep` and `gnused`. The shell needs no extra packages for them.
- **nixpkgs branches differ sharply for `openspec`.** `nixos-unstable` ships 1.13.1. `nixos-26.05` ships 1.4.1, which predates skills delivery and stores. `nixos-25.11` has no package.
- **nixpkgs cannot pin an exact Node version.** `nodejs_26` fixes the major version, and the lock picks the patch release.
- **One native npm module exists.** `better-sqlite3` is compiled for whichever Node version ran `npm install`.
- **The repo has 369 tracked files, about 15 MB in total.** Nix copies the tracked tree into its store when it evaluates a flake.
- **Nix is not yet installed on the maintainer's machine.** direnv is installed.

## Goals / Non-Goals

**Goals:**

- Keep the environment split into modules, so a shared flake can later take over the `openspec` module unchanged.
- Leave every existing route working: rustup, Node version managers, the `justfile` and the current CI workflows.

**Non-Goals:**

- Generating OpenSpec skills, or running any command when the shell starts. That is Track E stories E2 and E3.
- Any CI or production-image change beyond keeping every workflow's `node-version` and the `Dockerfile`'s base image in sync with `.nvmrc` (D6). Adding the skills-drift check job itself is story E4.
- Editor tooling such as `rust-analyzer` and `rust-src`. Adding them to `backend/rust-toolchain.toml` later would serve rustup users as well.
- Services such as PostgreSQL.
- A binary cache of our own. Every package comes prebuilt from cache.nixos.org.

## Decisions

### D1. Native flakes with flake-parts, not devenv

**Choice:** A root `flake.nix` built with flake-parts.

**Why:**
- Contributors and CI need only Nix, not a second wrapper CLI.
- `nix flake check` gives CI a standard place for checks.
- A flake-parts module is the natural unit to share between repos later.

**Alternatives:**
- **devenv:** friendlier module options and built-in services, but it adds a CLI that every contributor and CI job must install. Its flake mode is a reduced subset.
- **mise, devbox and flox:** rejected earlier. mise and devbox are not hermetic, and flox depends on a hosted service.

### D2. One nixpkgs input on `nixos-unstable`

**Choice:** A single nixpkgs input that follows `nixos-unstable`, pinned by `flake.lock`.

**Why:**
- It is the only branch with a current `openspec` that comes prebuilt.
- The lock pins one exact revision, so "unstable" affects only what a lock update brings in.
- A lock update arrives as a pull request, which a reviewer can test before merging.
- The branch choice doesn't touch the most important pins. Rust comes from rust-overlay (D3), and `nodejs_26` fixes Node's major version.

**Alternatives:**
- **`nixos-26.05` only:** its `openspec` 1.4.1 is too old.
- **Stable base plus unstable for `openspec` only:** safe but not needed. It costs a second nixpkgs download, a duplicate Node in the store, and a branch change every six months.
- **Stable plus building `openspec` ourselves:** every contributor would compile it, or we'd need a binary cache.

### D3. Rust from rust-overlay, read from the existing toolchain file

**Choice:** Build the toolchain with rust-overlay's `fromRustupToolchainFile ./backend/rust-toolchain.toml`.

**Why:**
- It reads the same file rustup reads, so the Rust version has one source of truth.
- It installs the official prebuilt Rust binaries and needs no extra hash.

**Alternatives:**
- **nixpkgs `rustc` and `cargo`:** can't pin an exact version.
- **fenix:** reads the toolchain file too, but needs a hash updated on every Rust bump. That breaks the single-source rule.

### D4. Three modules joined by two mergeable options

**Choice:** `flake.nix` declares inputs and systems, and imports three modules:

```
flake.nix         inputs: nixpkgs, flake-parts, rust-overlay
  |
  +-- nix/devshell.nix    declares options, builds devShells.default and checks.devshell
  |                         sidereal.shell.packages : list of packages (merged)
  |                         sidereal.shell.hooks    : lines of shell code (concatenated)
  |
  +-- nix/toolchains.nix  adds Rust, nodejs_26 and just       [specific to Sidereal]
  |
  +-- nix/openspec.nix    adds openspec                       [can be extracted later]
```

**Why:**
- Each module adds its own packages without editing `devshell.nix`.
- Story E3 will add a startup hook in `nix/openspec.nix`, and a later shared flake can add hooks the same way.
- The two options come to about ten lines of plain Nix, with no dependency on a third party.

**Alternatives:**
- **Explicit composition in one file:** simpler to read. However, every new module would mean editing the central file, which defeats extraction.
- **numtide/devshell or a make-shell flake module:** these do the same job but add a dependency and their own conventions.

**The option names use the `sidereal.` prefix.** A shared flake would later rename them to its own namespace in one small change.

### D5. `openspec` from nixpkgs as it stands

**Choice:** Use `pkgs.openspec` from the locked nixpkgs revision.

**Why:** It comes prebuilt, and the lock decides its version.

**When to override:** If we ever need a version nixpkgs doesn't have, `nix/openspec.nix` overrides the package with `overrideAttrs`, and only there. We would then weigh a binary cache.

### D6. Node: `nodejs_26`, and `.nvmrc` holds the major version

**Choice:** The shell uses `pkgs.nodejs_26`. `.nvmrc` becomes `26`. Every CI workflow reads it too, through `actions/setup-node`'s `node-version-file: '.nvmrc'` — not a separate hardcoded `node-version` string. The docs and `Dockerfile`'s base image still say Node 26 directly, since neither reads `.nvmrc`.

**Why:**
- Nix and Node version managers can agree only on the major version. A patch pin in `.nvmrc` would disagree with Nix after every lock update.
- Node 26 is what the maintainer's machine already runs. Picking it, rather than 24, makes the pinned shell match reality from the start, instead of asking the maintainer to switch down.
- nixpkgs already packages `nodejs_26` (26.10.0 on `nixos-unstable`), prebuilt.
- `node-version-file` follows the same single-source rule D3 applies to Rust: one file the CI workflows read, instead of four workflows each hardcoding their own copy of the version.

**Consequence:** Node 26 is in its Current phase, not yet Long-Term Support — Node typically promotes an even major to LTS in October of its release year. The pin may need to move again once it does. That next bump costs fewer edits than this one did: `nix/toolchains.nix`, `.nvmrc`, the docs, and `Dockerfile`'s two `FROM node:` lines. The four CI workflow files need no edit — they follow `.nvmrc` automatically through `node-version-file`.

Running production on Node 26 while it's still pre-LTS is itself a trade-off, weighed against keeping the production image on an older, more conservative major than dev and CI use. The maintainer chose consistency: one Node version everywhere, rather than a deliberate split between "what contributors and CI run" and "what production runs."

**Alternatives:**
- **`nodejs_24`, the LTS choice:** safer, but drifts from what the maintainer's machine — and every `npm install` run on it — already uses.
- **An exact version in `.nvmrc`** (`24.10.0`, or now some `26.x.y`): rejected either way. It disagrees with Nix's patch choice after every lock update, which is the drift this change exists to fix.

### D7. `.envrc`: guarded, pinned and watched

**Choice:** `.envrc` does these things, in this order:

1. Runs steps 2–4 only if the `nix` command exists. This is a condition around those steps, not an early exit, so step 5 always runs.
2. Loads nix-direnv 3.2.0 from its release URL, verified with a committed `sha256` hash, if the contributor hasn't installed nix-direnv.
3. Watches `backend/rust-toolchain.toml` and every file under `nix/`.
4. Runs `use flake`.
5. Loads `.envrc.local` last, if it exists. Git ignores that file, so each contributor can keep personal settings there.

**Why:**
- The condition in step 1 keeps `.envrc` harmless for contributors without Nix.
- Loading nix-direnv this way means contributors need only plain direnv.
- The hash check meets the rule that remote code is verified before it runs.
- nix-direnv 3.2.0's own watch list — `flake.nix`, `flake.lock`, `devshell.toml`, `~/.direnvrc`, `~/.config/direnv/direnvrc` and its generated cache file — never includes `backend/rust-toolchain.toml` or anything under `nix/`; its `direnvrc` source shows this. Without step 3, a change to the Rust pin or a module would not reload the shell.
- The repo now owns the `.envrc` name. Step 5 gives contributors who had their own `.envrc` somewhere to move it. It runs last so that a contributor's `PATH` or tool settings win over the development shell's.
- **`.envrc` never loads `.env`.** That file holds settings for the v0.10.x app, not for the development environment.

**Trust model:** direnv runs nothing until a contributor runs `direnv allow`. It asks again after any change to `.envrc`.

### D8. The shell is also a flake check

**Choice:** `checks.devshell` points at the development shell's derivation.

**Why:** `nix flake check` then builds the shell and fails if the shell is broken. Story E4 adds its checks in the same place.

### D9. Three systems, and only Linux tested

**Choice:** `systems = [ "x86_64-linux" "aarch64-linux" "aarch64-darwin" ]`.

**Why:**
- Linux gets tested locally now, and in CI from story E4.
- Apple Silicon Macs are best-effort: the inputs support them, but nobody tests them yet.
- Intel Macs are left out. nixpkgs support for them is winding down, and nobody has asked for them.

## Risks / Trade-offs

- **[Risk] Flakes see only files tracked by git.** A new file under `nix/` stays invisible to Nix until someone runs `git add` on it.
  → `CONTRIBUTING.md` says so in one line.
- **[Risk] `better-sqlite3` fails to load after switching to a different Node major version.** The v0.10.x server then crashes with a `NODE_MODULE_VERSION` error. This happens only across major versions: every Node 26.x release shares module version 147, so lock updates within 26 don't trigger it.
  → `CONTRIBUTING.md` lists `npm rebuild` as a one-time step for anyone who used another major version before entering the shell.
- **[Trade-off] The first cargo build in the shell rebuilds `backend/target` in full.** The version matches rustup's, but the compiler's path differs, so cargo's cached build data no longer matches.
  → This is a one-time cost, and the docs mention it.
- **[Risk] A lock update to `nixos-unstable` brings a broken or changed tool.**
  → Updates arrive only as pull requests. Reviewers run `nix flake check` and `nix develop --command just check` before merging. Story E4 adds this to CI.
- **[Risk] rustup's tools shadow the Nix tools.** This can happen if a contributor's shell setup puts `~/.cargo/bin` ahead of the Nix paths after direnv runs.
  → direnv puts the shell's paths first by default. The spec's version scenarios detect any shadowing.
- **[Trade-off] Nix copies about 15 MB of tracked files into its store on each evaluation.**
  → nix-direnv evaluates again only when a watched file changes, so this is rare.
- **[Risk] Nobody tests the macOS shell.**
  → The spec requires only that it is defined. Testing waits for a Mac contributor or a CI runner.

## Migration Plan

1. **Merging changes nothing for anyone who doesn't opt in.** The new files sit unused until a contributor runs `direnv allow` or `nix develop`.
2. **To opt in:**
   1. install Nix with flakes enabled;
   2. run `direnv allow` in the repo;
   3. run `npm rebuild` once.
3. **Rollback:** delete `flake.nix`, `flake.lock`, `nix/` and `.envrc`. No other file depends on them. The `.nvmrc` and documentation changes can stay, because they are correct without Nix.
