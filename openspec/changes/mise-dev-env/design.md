## Context

- The Nix flake is the only route that installs every pinned tool. `nix/toolchains.nix` takes Rust from `server/rust-toolchain.toml` (channel `stable`, with `rustfmt` and `clippy`) and adds Node 26, pnpm 12, `just` and OpenSSL. `nix/lets-spec.nix` imports Let's Spec's flake module, which adds `lets`, `openspec` and `gh` and a shell hook that runs `lets sync`.
- `flake.nix` follows Let's Spec's `main`. `flake.lock` pins commit `3a9e669`, which is not a release. mise can install only a tagged release.
- The `nix` workflow is the pattern to copy: path-filtered `pull_request` trigger, `contents: read`, one job, and a "Tool versions" step.
- Let's Spec v0.8.0's README gives the mise snippet: `node`, `gh`, `npm:@fission-ai/openspec` at the version the release notes name (1.13.2 for v0.8.0), `github:sidereal-io/lets-spec`, `LETS_SPEC_FLOW`, and `[hooks] enter = "lets sync"`. The hook runs only under `mise activate`.
- A prototype in `.workspace/` with mise 2026.10.17 showed:
  - From the repo root, mise does not read `server/rust-toolchain.toml`. It reads it only inside `server/`, so `mise.toml` must name Rust itself (D8).
  - mise reads `.nvmrc` only when the `idiomatic_version_file_enable_tools` setting lists `node`. The setting is empty by default. The user's global config enables it, which hides the gap on their machine.
  - `mise lock` with no flags writes seven platforms (`linux-x64`, `linux-arm64`, both as musl too, `macos-x64`, `macos-arm64`, `windows-x64`) for every downloaded tool. Rust is recorded as `version = "stable"`, with no platforms. The npm tool has no platforms.
  - For an npm tool, `mise lock` also writes a dependency lock beside `mise.lock`: `.mise/locks/npm-fission-ai-openspec/<version>/package.json` and `aube-lock.yaml`. `mise.lock` records its path and digest. `mise install --locked` fails when that directory is missing or does not match.
  - A lock entry applies only while its `specifiers` match the request in `mise.toml`. Edit the request without re-running `mise lock`, and plain `mise install` ignores the entry. `mise install --locked` fails instead.

## Goals / Non-Goals

**Goals:**

- A contributor with mise runs `mise trust` and `mise install` once, and gets every tool at the release `mise.lock` pins, with Rust following rustup's stable channel.
- Both routes run one Let's Spec release, so `lets sync --check` passes whichever route wrote the copies.
- CI proves the mise route installs from the lock, on every pull request that changes it.

**Non-Goals:**

- Moving `ci` workflow jobs or `justfile` recipes onto mise.
- Removing or changing the Nix shell, except for the Let's Spec release it pins.
- A CI check that fails when the lock is missing a platform, and a weekly update workflow. Later stories own these.
- Pinning mise's own version.

## Test Seams

- **Primary test seam:** the `mise` workflow run on this change's pull request. It installs from the lock, checks the Rust channels, and prints the seven versions. `mise.toml` is new, so the workflow runs.
- **Existing seams reused:** `just check` (its `check-openspec` step validates the change); the `ci` workflow's `lets-spec` job, which runs `lets sync --check` through the Nix shell; and the `nix` workflow, which runs because `flake.nix` and `flake.lock` change. The spec's scenarios are shell commands the builder runs locally; each task names the one it proves.

## Scrutiny Tier & Blast Radius

- **Tier:** high. The change adds a new external dependency (mise and `jdx/mise-action`), supersedes a decision record, and adds a hook that runs code when a contributor enters the repo.
- **Blast radius:** contributor tooling and CI only. No server, web or database code changes. The Let's Spec move rewrites the committed `lets-*` skills, `lets-*` schemas and the rules block in `AGENTS.md`, which every agent session reads.

## Decisions

### 1. The Nix route moves to Let's Spec v0.8.0 first, in its own commit (D1, D2)

`flake.nix` changes `lets-spec.url` to `github:sidereal-io/lets-spec/v0.8.0`, and its comment says the tag stays pinned until the Nix route goes. Then `nix flake update lets-spec`, then `nix develop --command true` so the shell hook runs `lets sync`. One commit holds `flake.nix`, `flake.lock` and every file `lets sync` wrote. It comes before the mise files, so the mise route arrives on a tree that already matches v0.8.0.

Alternative: pin the flake to the untagged commit and make mise match it. mise cannot install an untagged commit, so the routes could never agree.

### 2. `mise.toml` names every tool except Node

```toml
[tools]
rust = { version = "stable", components = "rustfmt,clippy" }
pnpm = "12"
just = "1"
gh = "2"
"npm:@fission-ai/openspec" = "1.13.2"
"github:sidereal-io/lets-spec" = "0.8.0"

[env]
LETS_SPEC_FLOW = "standard"
OPENSPEC_NO_UPDATE_CHECK = "1"

[hooks]
enter = "lets sync"

[settings]
idiomatic_version_file_enable_tools = ["node"]
```

- **Rust** names the `stable` channel with both components (D8). mise installs it through rustup and sets `RUSTUP_TOOLCHAIN`, which wins over `server/rust-toolchain.toml` inside `server/`. The channels match, so the result is the same.
- **Node** has no entry. The `[settings]` line makes mise read `.nvmrc` for every contributor, whatever their global config says. Naming `node = "26"` instead would put a second copy of the Node version in the repo, which the "one Node version" requirement exists to prevent.
- **pnpm, `just` and `gh`** name a major version. The lock pins the exact release, and a later `mise upgrade` can move within the major. `gh` 2 resolves to a release well past 2.102.0, the minimum Let's Spec needs.
- **`openspec` and Let's Spec** name exact releases. They must move together, because each Let's Spec release names the CLI its skills were generated from. Pinning both exactly makes moving one a deliberate edit.
- **`[env]`** sets the flow and keeps the CLI offline (D5). `LETS_SPEC_SKILLS` is left unset: `committed` is the default.

### 3. `mise.lock` and its npm sidecar are committed

The builder runs `mise lock` with no flags (D6) and commits `mise.lock` and `.mise/locks/`. Without the sidecar, `mise install --locked` fails, and a plain install would resolve `openspec`'s dependencies afresh. A sidecar change always comes with a digest change in `mise.lock`, so the workflow's `mise.lock` trigger (D4) covers it.

### 4. `.gitignore` ignores the personal files

Add `mise.local.toml`, and also `mise.local.lock` and `.mise/locks/mise.local/`, which mise writes when a personal file names tools. Without them, a contributor's own tools would show in `git status`.

### 5. The `mise` workflow installs with `--locked`

`.github/workflows/mise.yml`, copying `nix.yml`'s shape:

- `pull_request` on `mise.toml`, `mise.lock`, `server/rust-toolchain.toml`, `.nvmrc` and `.github/workflows/mise.yml` (D4); the same `concurrency` block; `permissions: contents: read`.
- `actions/checkout@v7`, then `jdx/mise-action@v5` with `install_args: --locked`. The action's default `github_token` is the job's own token, not a repository secret, and it keeps GitHub API calls under the rate limit.
- A "Rust channels agree" step: read `mise config get tools.rust.version` and the `channel` line of `server/rust-toolchain.toml`. If they differ, print one error line naming both files and their values, and exit 1.
- A "Tool versions" step: `mise exec -- sh -c 'rustc --version && node --version && pnpm --version && just --version && gh --version && openspec --version && lets --version'`.

`--locked` turns a stale lock into a failure: an edited request with no new `mise lock`, a missing sidecar, or a wrong checksum. A plain install would quietly resolve a fresh release instead. If `--locked` rejects the Rust entry (it has no download address), the builder drops `--locked` from the action and keeps the plain install. This also goes in the pull request description.

### 6. The decision record supersedes ADR-013 (D3)

A new record, Accepted, chooses mise. Its Context gives #496's four reasons. Its Consequences say that rustup's newest stable release decides Rust on the mise route, and that no file names a Rust release. ADR-013's status becomes superseded, with a link to the new record. The ADR step writes both.

### 7. Docs: a short mise section, and the version table names both locks (D7)

- `CONTRIBUTING.md` gets a "With mise" section beside "With Nix": install mise, add `mise activate` to the shell, run `mise trust` and `mise install`, and keep personal settings in `mise.local.toml`. It warns against using both routes in one shell, and against loading an untrusted branch, as the direnv warning does.
- The "Where versions are set" table names `mise.toml` and `mise.lock` in the rows for Rust, `just`, pnpm and Let's Spec. The Node row says mise reads `.nvmrc`. The lead sentence "Each version lives in one file" changes to say that, until the Nix route goes, some versions are set in both locks and change together.
- The Let's Spec update paragraph in `CONTRIBUTING.md`, and "Update Let's Spec through the flake" in `AGENTS.md`, change to: edit the tag in `flake.nix` and both versions in `mise.toml`, run `nix flake update lets-spec` and `mise lock`, enter the shell, and commit everything. Dependabot no longer moves Let's Spec, because the input names a tag.
- `AGENTS.md`'s Rust line adds that rustup decides the release on the mise route.
- No doc names the `openspec` version: `1.13.2` matches the spec's Rust-release search. The docs point to the release notes, as now.

## Risks / Trade-offs

- [The `enter` hook runs code from the working tree when a contributor enters the repo. mise trusts a config by its path, not its content, so a checkout of another branch runs that branch's hook without asking.] → `CONTRIBUTING.md` tells the contributor to run `mise trust --untrust` before checking out a branch they don't trust, as it does for direnv.
- [A contributor with both direnv's Nix shell and `mise activate` gets two sets of tools on `PATH`, in whichever order the hooks ran.] → The docs say to use one route per shell. The overlap ends when the Nix route goes.
- [mise's own version is not pinned. A contributor with an old mise may fail to read the lock, or rewrite it in an older format.] → The docs ask for a recent mise. CI uses the action's latest mise, so a lock that only a newer mise can read fails loudly in CI.
- [On the mise route, two contributors can have different Rust releases, whatever rustup last installed.] → Accepted (D3). It is the same trade-off rustup users have today. CI prints the release it used.
- [The v0.8.0 sync rewrites about 43 skill, schema and rules files that agents read.] → Its own commit, so a reviewer can read it apart from the mise work. The `ci` workflow's `lets-spec` job checks the result.
- [The workflow downloads from GitHub's API for the `github:` and `aqua` backends.] → The job's own token lifts the rate limit, with read-only permission.

## Migration & Rollback

No data changes shape. Contributors on the Nix route do nothing: the next shell load runs the v0.8.0 sync, which finds the committed copies already current.

To roll back, revert the pull request. That restores the untagged flake pin and the v0.7-era skill copies, and removes `mise.toml`, `mise.lock`, `.mise/locks/` and the workflow. A contributor who installed through mise keeps the tools in mise's own directories until they run `mise prune`.

## Open Questions

- Whether `mise install --locked` accepts the Rust entry. Decision 5 says what the builder does in either case.
