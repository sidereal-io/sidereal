# dev-environment Specification

## Purpose

The development environment gives every contributor and AI agent the same pinned tools for this repo. It turns on automatically when Nix is installed. It stays optional, so contributors without Nix can still build and check the repo. It also generates the same agent skills on every machine, from settings tracked in the repo.

## Requirements

### Requirement: The shell provides the pinned toolchain

The development shell SHALL provide these tools on `PATH`:

- Rust at a stable release, with `rustfmt` and `clippy`;
- Node at major version 26;
- pnpm at major version 12;
- `just`;
- the `openspec` CLI.

#### Scenario: A contributor checks tool versions in the shell

- **WHEN** a contributor runs `nix develop --command sh -c 'rustc --version; node --version; pnpm --version; just --version; openspec --version'` at the repo root
- **THEN** `rustc --version` output starts with `rustc 1.`
- **AND** the `rustc --version` output contains neither `beta` nor `nightly`
- **AND** `node --version` output starts with `v26.`
- **AND** `pnpm --version` output starts with `12.`
- **AND** the `just` and `openspec` commands each exit with status 0

#### Scenario: Formatting and lint tools come with Rust

- **WHEN** a contributor runs `nix develop --command sh -c 'cargo fmt --version && cargo clippy --version'`
- **THEN** both commands exit with status 0

### Requirement: Rust follows the latest stable release

`server/rust-toolchain.toml` SHALL name the `stable` channel and no exact release. The development shell SHALL use the newest stable release that its locked inputs know. A new Rust release SHALL reach the development shell only through a change to `flake.lock`.

#### Scenario: The toolchain file names the stable channel

- **WHEN** a reviewer runs `grep -n '^channel' server/rust-toolchain.toml`
- **THEN** the output is exactly one line, `channel = "stable"`, after its line number

#### Scenario: A lock update brings a new Rust release

- **WHEN** a pull request changes only `flake.lock`, and the new lock knows a newer stable Rust release than the old lock
- **THEN** `nix develop --command rustc --version`, run on that pull request's commit, reports the newer release

#### Scenario: The same lock gives the same Rust release

- **WHEN** a tester runs `nix develop --command rustc --version` on one commit, on two days with different latest Rust releases
- **THEN** both runs report the same release

### Requirement: No file names a Rust release

The repo SHALL name no Rust release number in its build files or its contributor docs. The only Rust version settings SHALL be the `stable` channel in `server/rust-toolchain.toml` and the `stable` channel in `mise.toml`. `mise.lock` SHALL record Rust as the `stable` channel, not as a release. The app image SHALL name only Rust major version 1 in its base image, and SHALL still build.

#### Scenario: The Cargo manifest sets no minimum Rust version

- **WHEN** a reviewer runs `grep -rn 'rust-version' server --include=Cargo.toml`
- **THEN** the search finds no match

#### Scenario: The mise configuration names the stable channel

- **WHEN** a reviewer runs `mise config get tools.rust.version` at the repo root
- **THEN** the output is `stable`

#### Scenario: The mise lock records the channel, not a release

- **WHEN** a reviewer reads the `[[tools.rust]]` entry in `mise.lock`
- **THEN** its `version` is `stable`
- **AND** its `specifiers` list is `["stable"]`

#### Scenario: The server image names only the major version

- **WHEN** a reviewer runs `grep -n '^FROM rust:' Dockerfile` at the repo root
- **THEN** the search finds at least one line
- **AND** every matching line names an image tag that starts with `rust:1-`

#### Scenario: The server image builds

- **WHEN** a reviewer runs `docker build .` at the repo root
- **THEN** the command exits with status 0

#### Scenario: The contributor docs name no Rust release

- **WHEN** a reviewer runs `grep -nE '\b1\.[0-9]{2}(\.[0-9]+)?\b' AGENTS.md README.md server/README.md CONTRIBUTING.md`
- **THEN** the search finds no match

### Requirement: The lock file decides every tool version

The development shell SHALL resolve every tool from inputs pinned in the committed `flake.lock`. The same commit SHALL produce the same shell on every machine of the same system type.

#### Scenario: Two clones of one commit agree

- **WHEN** a tester clones the same commit into two different directories on one machine
- **AND** runs `nix eval --raw .#devShells.<system>.default.drvPath` in each clone
- **THEN** both commands print an identical derivation path

### Requirement: The environment verifies remote code before running it

The environment SHALL verify every piece of remote code it fetches against a hash committed to the repo. This covers flake inputs and any script that `.envrc` loads. The environment SHALL refuse to use content whose hash does not match.

#### Scenario: A fetched script does not match its hash

- **WHEN** `.envrc` fetches a remote script whose content differs from the hash committed in `.envrc`
- **THEN** direnv stops with an error and does not run the script

#### Scenario: Every flake input is locked

- **WHEN** a reviewer runs `nix flake metadata --json` at the repo root
- **THEN** every input in the output has a `locked` entry with a `narHash`

### Requirement: The shell turns on when you enter the repo

For a contributor with Nix and direnv, entering the repo directory SHALL turn the development shell on after a one-time `direnv allow`. Leaving the directory SHALL restore the contributor's previous environment. The shell SHALL reload when a file that defines it changes. The shell SHALL also reload when a file that the skills are generated from changes: `.config/openspec/config.json` or `scripts/skills.sh`.

#### Scenario: A contributor enters and leaves the repo

- **WHEN** a contributor with Nix and direnv has run `direnv allow` once, then changes into the repo directory
- **THEN** `rustc --version` reports the same release as `nix develop --command rustc --version`, without any further command
- **AND** after the contributor changes to a directory outside the repo, `rustc --version` reports the same result as before they entered it

#### Scenario: The Rust pin changes while the shell is active

- **WHEN** a tester with the shell loaded changes `channel` in `server/rust-toolchain.toml` from `stable` to `1.97.1`, without leaving the repo directory
- **AND** runs `direnv export bash` in the repo directory
- **THEN** `rustc --version` in that shell reports 1.97.1, not the release from before the edit

#### Scenario: The watch list covers the shell's files

- **WHEN** a tester with the shell loaded runs `direnv status` in the repo directory
- **THEN** the output lists `server/rust-toolchain.toml` and each file under `nix/` as watched
- **AND** the output lists `.config/openspec/config.json` and `scripts/skills.sh` as watched

### Requirement: Contributors can add their own direnv settings

`.envrc` SHALL load a file named `.envrc.local` when it exists, whether or not Nix is installed. It SHALL load `.envrc.local` last, after the development shell, so the contributor's settings take precedence. Git SHALL ignore `.envrc.local`, so each contributor keeps their own settings out of commits.

#### Scenario: A contributor adds a personal variable

- **WHEN** a contributor writes `export SIDEREAL_TEST_VAR=1` in `.envrc.local` and enters the repo directory
- **THEN** `SIDEREAL_TEST_VAR` equals `1` in the contributor's shell
- **AND** the result is the same with Nix installed and without it

#### Scenario: A personal setting overrides the development shell

- **WHEN** a contributor with Nix writes `export PATH="$PWD/.local-bin:$PATH"` in `.envrc.local` and enters the repo directory
- **THEN** the first entry of `PATH` in the contributor's shell is the repo's `.local-bin` directory

#### Scenario: The personal file stays out of git

- **WHEN** a contributor creates `.envrc.local` and runs `git status --porcelain`
- **THEN** the output does not list `.envrc.local`

### Requirement: Nix stays optional

The environment SHALL NOT require Nix for building or checking the repo. For a contributor with direnv but without Nix, entering the repo SHALL produce no error. It SHALL change no environment variables except direnv's own `DIRENV_*` variables and any that the contributor's `.envrc.local` sets. The repo SHALL keep the pin files that tools outside Nix read: `server/rust-toolchain.toml` for rustup, and `.nvmrc` for Node version managers.

#### Scenario: A contributor has direnv but not Nix

- **WHEN** a contributor without Nix and without `.envrc.local` runs `direnv allow`, then enters the repo directory
- **THEN** direnv reports no error
- **AND** `env`, with lines starting `DIRENV_` removed, shows the same output as before the contributor entered the directory

#### Scenario: A contributor without Nix runs the checks

- **WHEN** a contributor without Nix installs Rust through rustup and installs Node through a version manager reading `.nvmrc`
- **THEN** rustup selects the channel in `server/rust-toolchain.toml`
- **AND** the version manager selects Node major version 26
- **AND** `just check` exits with status 0, given that `just` is installed

#### Scenario: The environment does not load application settings

- **WHEN** a contributor enters the repo directory with the development shell turned on
- **THEN** no variable defined only in `.env` is set in the contributor's shell

### Requirement: The repo checks pass inside the shell

The repo's existing gate SHALL pass inside the development shell. The gate is `just check`: `cargo fmt --check`, clippy with warnings denied, `cargo test`, and the dependency-direction lint.

#### Scenario: A contributor runs the gate in the shell

- **WHEN** a contributor runs `nix develop --command just check` on a commit whose checks pass outside Nix
- **THEN** the command exits with status 0

### Requirement: The flake check builds the shell

`nix flake check` SHALL build the development shell. It SHALL fail when the shell cannot be built.

#### Scenario: The shell definition is broken

- **WHEN** a change makes the development shell impossible to build, and a reviewer runs `nix flake check`
- **THEN** `nix flake check` exits with a non-zero status

#### Scenario: The shell definition is sound

- **WHEN** a reviewer runs `nix flake check` on a commit whose shell builds
- **THEN** `nix flake check` exits with status 0

### Requirement: The shell is defined for the supported systems

The flake SHALL define a development shell for `x86_64-linux`, `aarch64-linux` and `aarch64-darwin`.

#### Scenario: A contributor lists the development shells

- **WHEN** a contributor runs `nix flake show --json --all-systems`
- **THEN** the output contains `devShells.x86_64-linux.default`, `devShells.aarch64-linux.default` and `devShells.aarch64-darwin.default`

### Requirement: The skills command gives the same skills on every machine

Running `just skills` SHALL generate the OpenSpec skills from settings the repo defines. For a given CLI version, the result SHALL NOT depend on the contributor's global OpenSpec config, or on generated files left from an earlier run. The command SHALL generate these 8 workflows, as skills only: `propose`, `explore`, `continue`, `apply`, `update`, `sync`, `archive` and `verify`. The generated skills SHALL use only the generic `/openspec-<name>` form to name other skills.

#### Scenario: Two machines with different global configs agree

- **WHEN** a tester runs `just skills` in two clones of one commit, with the same `openspec` version
- **AND** the two runs use different global OpenSpec configs, for example one with the `core` profile and one with `delivery` set to `both`
- **THEN** `diff -r .agents/skills` between the two clones reports no difference

#### Scenario: The expected workflows are generated

- **WHEN** a contributor runs `just skills`
- **THEN** `.agents/skills` contains exactly these generated folders: `openspec-propose`, `openspec-explore`, `openspec-continue-change`, `openspec-apply-change`, `openspec-update-change`, `openspec-sync-specs`, `openspec-archive-change` and `openspec-verify-change`
- **AND** the run creates or changes no file outside `.agents/skills`

#### Scenario: A stale Codex marker is left in the clone

- **WHEN** `.agents/skills/.openspec-target` contains `codex`, and a contributor runs `just skills`
- **THEN** no file under `.agents/skills` contains the text `$openspec-`
- **AND** the generated skills match those from a clone that never had the marker

#### Scenario: A workflow is removed from the repo settings

- **WHEN** a maintainer removes `verify` from the workflow list in `.config/openspec/config.json` and runs `just skills`
- **THEN** `.agents/skills/openspec-verify-change` no longer exists

### Requirement: The skills command leaves the contributor's own config alone

`just skills` SHALL NOT read or change the contributor's global OpenSpec config. It SHALL redirect the CLI to the repo's settings for its own run only, so the contributor's shell keeps its environment. It SHALL run the CLI with usage telemetry turned off. It SHALL NOT delete or write any file outside the repo.

#### Scenario: The global config file is unchanged

- **WHEN** a contributor records a checksum of `~/.config/openspec/config.json`, then runs `just skills`
- **THEN** the checksum of that file is unchanged

#### Scenario: The shell environment is unchanged

- **WHEN** a contributor runs `just skills` in an interactive shell
- **THEN** `XDG_CONFIG_HOME` and `OPENSPEC_TELEMETRY` in that shell keep the values they had before the run

#### Scenario: An agent folder links outside the repo

- **WHEN** `.agents` is a symbolic link to a directory outside the repo, and a contributor runs `just skills`
- **THEN** the command exits with a non-zero status
- **AND** no file in the link's target directory changes

#### Scenario: The CLI does not write to the repo's settings

- **WHEN** a contributor runs `just skills`
- **THEN** `git diff --exit-code .config/openspec/config.json` exits with status 0

### Requirement: The Nix shell provides the OpenSpec CLI

The Nix development shell SHALL provide the `openspec` CLI. Its lock file SHALL be the only pin for the CLI version. `just skills` SHALL stop before changing any file when `openspec` is not on `PATH`, and its error message SHALL name the command that installs it. Every generated `SKILL.md` SHALL record the CLI version that generated it.

#### Scenario: The CLI is missing

- **WHEN** a contributor without `openspec` on `PATH` runs `just skills`
- **THEN** the command exits with a non-zero status
- **AND** its error message names `npm install -g @fission-ai/openspec`
- **AND** `git status --porcelain` shows the same output as before the run

#### Scenario: Generated skills record the CLI version

- **WHEN** a contributor runs `just skills`
- **THEN** every generated `SKILL.md` has `generatedBy` equal to the output of `openspec --version`

#### Scenario: The Nix shell provides the CLI

- **WHEN** a contributor runs `nix develop --command just skills`
- **THEN** the command exits with status 0

### Requirement: Git tracks authored skills and the skill settings

Git SHALL track the skills people write and the skill settings in `.config/openspec/config.json`. Git SHALL NOT track generated skills or the `.openspec-target` marker. The prefix `openspec-` SHALL be reserved for generated skills. `just skills` SHALL NOT change any skill whose folder name lacks that prefix.

#### Scenario: Generating skills leaves git clean

- **WHEN** a contributor runs `just skills` on a clean checkout
- **THEN** `git status --porcelain` prints nothing

#### Scenario: No generated skill is tracked

- **WHEN** a reviewer runs `git ls-files .agents/skills`
- **THEN** no path in the output starts with `.agents/skills/openspec-`
- **AND** the output does not include `.agents/skills/.openspec-target`

#### Scenario: The skill settings are tracked

- **WHEN** a reviewer runs `git ls-files .config/openspec`
- **THEN** the output is `.config/openspec/config.json`

#### Scenario: Authored skills survive generation

- **WHEN** a contributor runs `just skills`
- **THEN** every file under `.agents/skills/peer-code-review`, `.agents/skills/grill-me`, `.agents/skills/peer-plan-review` and `.agents/skills/discovery` is unchanged, byte for byte

#### Scenario: Running the command twice changes nothing

- **WHEN** a contributor runs `just skills` twice in a row
- **THEN** no file under `.agents/skills` differs between the two results

### Requirement: Every supported agent sees the same skills

Claude Code SHALL see the same skill set as agents that read `.agents/skills`. That set SHALL include both the generated and the authored skills. The repo SHALL keep one copy of each skill.

#### Scenario: Claude sees the generated and authored skills

- **WHEN** a contributor runs `just skills`
- **THEN** `.claude/skills/openspec-propose/SKILL.md` and `.claude/skills/peer-code-review/SKILL.md` both exist
- **AND** `.claude/skills` resolves to the same directory as `.agents/skills`

### Requirement: Skills work without Nix

A contributor without Nix SHALL be able to generate the skills with `just` and an `openspec` CLI installed through npm. With the same CLI version, the result SHALL match the result inside the Nix shell.

#### Scenario: A contributor without Nix generates skills

- **WHEN** a contributor without Nix installs `just` and `openspec` through npm, then runs `just skills`
- **THEN** the command exits with status 0
- **AND** `.agents/skills` contains the 8 generated folders
- **AND** no file under `.agents/skills` contains the text `$openspec-`

#### Scenario: The same CLI version gives the same skills

- **WHEN** a contributor without Nix installs the `openspec` version that the Nix shell provides, then runs `just skills`
- **THEN** `diff -r .agents/skills` against a run inside the Nix shell reports no difference

### Requirement: Loading the shell refreshes stale skills

When the Nix development shell loads, it SHALL regenerate the OpenSpec skills if they are stale. The skills are stale when any of these is true:

- no successful `just skills` run has recorded what it generated;
- the `openspec` version differs from the version that generated them;
- `.config/openspec/config.json` or `scripts/skills.sh` differs from the version used to generate them;
- a generated skill folder from that run is missing.

When none is true, loading the shell SHALL NOT change any file under `.agents/skills`. Running `just skills` by hand SHALL keep working as before.

#### Scenario: A fresh clone gets its skills

- **WHEN** a contributor with Nix and direnv clones the repo, runs `direnv allow`, and enters the repo directory
- **THEN** `.agents/skills` contains the 8 generated `openspec-*` folders
- **AND** `git status --porcelain` prints nothing

#### Scenario: Nothing changed since the last refresh

- **WHEN** a tester records the modification time of every file under `.agents/skills`, leaves the repo directory, and enters it again
- **THEN** every modification time is unchanged

#### Scenario: The skill settings changed

- **WHEN** a maintainer removes `verify` from the workflow list in `.config/openspec/config.json`
- **AND** the shell loads again
- **THEN** `.agents/skills/openspec-verify-change` no longer exists

#### Scenario: The CLI version changed

- **WHEN** the shell loads with an `openspec` whose version differs from the `generatedBy` value in the generated skills
- **THEN** every generated `SKILL.md` has `generatedBy` equal to the output of `openspec --version`

#### Scenario: A generated skill folder was deleted

- **WHEN** a contributor deletes `.agents/skills/openspec-explore`, then the shell loads again
- **THEN** `.agents/skills/openspec-explore/SKILL.md` exists

### Requirement: Refreshing skills never blocks the shell

A failed refresh SHALL NOT stop the development shell from loading. A successful refresh, or a skipped one, SHALL print nothing. A failed refresh SHALL print one warning line to standard error. The line SHALL name `just skills` as the way to retry. The hook SHALL run `just enter` only when the repo's `justfile` defines an `enter` recipe, and SHALL print nothing when it does not.

#### Scenario: The refresh fails

- **WHEN** the skills are stale, and `openspec` on `PATH` is a stub that exits with status 1
- **AND** a tester runs `nix develop --command true` in the repo directory
- **THEN** the command exits with status 0
- **AND** standard error contains exactly one line that names `just skills`

#### Scenario: The refresh succeeds

- **WHEN** the skills are stale, and a tester runs `nix develop --command true` in the repo directory
- **THEN** the command exits with status 0
- **AND** the hook writes nothing to standard output or standard error

#### Scenario: The repo has no enter recipe

- **WHEN** a tester removes the `enter` recipe from the `justfile`, then runs `nix develop --command true`
- **THEN** the command exits with status 0
- **AND** the hook writes nothing to standard output or standard error

### Requirement: The shell keeps the OpenSpec CLI offline

The development shell SHALL set `OPENSPEC_NO_UPDATE_CHECK=1`, so no `openspec` command run in the shell checks the npm registry for a newer version. Refreshing the skills SHALL need no network access.

#### Scenario: The shell turns off the update check

- **WHEN** a tester runs `nix develop --command printenv OPENSPEC_NO_UPDATE_CHECK`
- **THEN** the output is `1`

#### Scenario: The refresh works without a network

- **WHEN** the skills are stale, and a tester loads the shell with no network access, for example inside `unshare --net --map-root-user` on Linux
- **THEN** the refresh completes, and `.agents/skills` contains the 8 generated `openspec-*` folders
- **AND** the refresh opens no network connection

### Requirement: Contributors without Nix can opt in to the refresh

A contributor without Nix SHALL be able to get the same refresh by running `just enter` from `.envrc.local`. Without that opt-in, the environment SHALL keep its current behavior for contributors without Nix.

#### Scenario: A contributor without Nix opts in

- **WHEN** a contributor without Nix, with `just` and `openspec` installed, writes `just enter` in `.envrc.local`
- **AND** deletes `.agents/skills/openspec-explore`, then enters the repo directory
- **THEN** `.agents/skills/openspec-explore/SKILL.md` exists

### Requirement: Dependabot proposes Nix pin updates every week

Dependabot SHALL check the inputs in `flake.lock` every week. When an input has a newer upstream commit, Dependabot SHALL open a pull request that updates it. The configuration SHALL ask Dependabot to group all Nix inputs into one pull request.

#### Scenario: The configuration declares the Nix updates

- **WHEN** a reviewer runs `yq '.updates[] | select(."package-ecosystem" == "nix")' .github/dependabot.yml`
- **THEN** the output is exactly one entry
- **AND** that entry has `directory` set to `/` and `schedule.interval` set to `weekly`
- **AND** that entry has a group whose `patterns` list is `["*"]`

#### Scenario: A flake input has a newer commit

- **WHEN** Dependabot's weekly run finds a newer upstream commit for an input in `flake.lock`
- **THEN** an open pull request from Dependabot changes `flake.lock`
- **AND** that pull request changes no other file

### Requirement: Dependabot proposes Cargo updates every week

Dependabot SHALL check the crates in `server/Cargo.lock` every week. When a crate has a newer release, Dependabot SHALL open one grouped pull request for all the updated crates.

#### Scenario: The configuration declares the Cargo updates

- **WHEN** a reviewer runs `yq '.updates[] | select(."package-ecosystem" == "cargo")' .github/dependabot.yml`
- **THEN** the output is exactly one entry
- **AND** that entry has `directory` set to `/server` and `schedule.interval` set to `weekly`
- **AND** that entry has a group whose `patterns` list is `["*"]`

#### Scenario: Several crates have newer releases

- **WHEN** Dependabot's weekly run finds newer releases for two or more crates
- **THEN** one open pull request from Dependabot updates all of them
- **AND** that pull request changes files under `server/` only

### Requirement: Dependabot proposes web package updates every week

Dependabot SHALL check the packages in `web/package.json` and `web/pnpm-lock.yaml` every Monday at 04:00. When a package has a newer release, Dependabot SHALL open a pull request that updates it. The `/web` entry SHALL group updates exactly as the root `npm` entry does. Dependabot SHALL NOT propose a TypeScript major update for `web/`.

The scenarios below use `yq` version 4 (the Go implementation). The development shell does not provide it, so a reviewer installs it first.

#### Scenario: The configuration declares the web updates

- **WHEN** a reviewer runs `yq '.updates[] | select(."package-ecosystem" == "npm" and .directory == "/web")' .github/dependabot.yml`
- **THEN** the output is exactly one entry
- **AND** that entry has `schedule.interval` set to `weekly`, `schedule.day` set to `monday`, and `schedule.time` set to `04:00`

#### Scenario: The web entry groups updates like the root entry

- **WHEN** a reviewer runs `yq '.updates[] | select(."package-ecosystem" == "npm" and .directory == "/web") | .groups' .github/dependabot.yml`
- **AND** the reviewer runs the same command with `.directory == "/"`
- **THEN** the two outputs are identical

#### Scenario: The web entry ignores TypeScript major updates

- **WHEN** a reviewer runs `yq '.updates[] | select(."package-ecosystem" == "npm" and .directory == "/web") | .ignore' .github/dependabot.yml`
- **THEN** the output is exactly one rule
- **AND** that rule has `dependency-name` set to `typescript` and `update-types` set to `["version-update:semver-major"]`

#### Scenario: A web package has a newer release

- **WHEN** Dependabot's weekly run finds a newer release for a package in `web/package.json`
- **THEN** an open pull request from Dependabot changes `web/pnpm-lock.yaml`
- **AND** that pull request changes files under `web/` only
- **AND** the `ci` workflow's `web` job runs on that pull request

#### Scenario: TypeScript has a newer major release

- **WHEN** Dependabot's weekly run finds a TypeScript release with a higher major version than the one in `web/pnpm-lock.yaml`
- **THEN** no open pull request from Dependabot changes the major version of `typescript` in `web/package.json`

### Requirement: CI builds the shell on every pull request that changes it

A CI job, the flake check, SHALL run `nix flake check` on every pull request that changes one of these files:

- `flake.nix` or `flake.lock`;
- any file under `nix/`;
- `server/rust-toolchain.toml`;
- the workflow file that defines the flake check.

The flake check SHALL fail when the shell cannot be built.

#### Scenario: A pull request breaks the shell

- **WHEN** a pull request changes `flake.nix` so that the development shell cannot be built
- **THEN** the flake check on that pull request fails

#### Scenario: A pull request leaves the shell working

- **WHEN** a pull request changes `flake.lock` and the development shell still builds
- **THEN** the flake check on that pull request passes

#### Scenario: A pull request does not touch the shell

- **WHEN** a pull request changes none of the files in this requirement's list, including the flake check's own workflow file
- **THEN** GitHub does not run the flake check on that pull request

### Requirement: The flake check runs without secrets

The flake check SHALL use no repository secret. Its workflow SHALL grant the job read-only access to repository contents and no other permission. A Dependabot pull request SHALL therefore run the check with no extra setup.

#### Scenario: A reviewer inspects the workflow permissions

- **WHEN** a reviewer reads the flake check workflow file
- **THEN** its `permissions` block grants only `contents: read`
- **AND** the file contains no reference to `secrets.`

#### Scenario: Dependabot opens a Nix pin update

- **WHEN** Dependabot opens a pull request that changes `flake.lock`
- **THEN** the flake check runs on that pull request and reports pass or fail

### Requirement: The flake check reports the shell's tool versions

After the shell builds, the flake check SHALL run `rustc --version`, `openspec --version`, `node --version`, `pnpm --version`, and `just --version` inside the shell. A reviewer SHALL be able to read their output in the job log. The flake check SHALL fail when any of the five commands fails.

#### Scenario: A reviewer checks what a pin update changed

- **WHEN** the flake check passes on a pull request
- **THEN** the job log contains the output of `rustc --version`, `openspec --version`, `node --version`, `pnpm --version`, and `just --version`, each run inside the development shell

#### Scenario: A pin update breaks a tool

- **WHEN** `openspec --version` exits with a non-zero status inside the shell, and the other four commands succeed
- **THEN** the flake check fails

### Requirement: Contributors and CI name one Node version

The repo SHALL state Node major version 26 wherever it names a Node version for contributors. `.nvmrc` SHALL contain the major version only, so Nix and Node version managers agree. A CI workflow that sets up Node outside the development shell SHALL read `.nvmrc` rather than hold its own copy of the version, so CI can never drift from it. The app image SHALL build the web shell with a Node base image of the major version that `.nvmrc` names.

#### Scenario: A reviewer checks the documented Node version

- **WHEN** a reviewer reads `.nvmrc`
- **THEN** the file contains `26` and nothing else except a trailing newline

#### Scenario: No outdated Node version is left in the docs

- **WHEN** a reviewer searches `AGENTS.md`, `README.md`, `server/README.md`, `web/README.md` and `CONTRIBUTING.md` for a Node major version other than 26
- **THEN** the search finds no match

#### Scenario: A CI workflow that sets up Node reads .nvmrc

- **WHEN** a reviewer runs `grep -rn node-version .github/workflows/`
- **THEN** every matching line reads `node-version-file: '.nvmrc'`
- **AND** no matching line hardcodes a Node version number

#### Scenario: The app image builds the web shell with the same Node version

- **WHEN** a reviewer runs `grep -n '^FROM node:' Dockerfile` at the repo root
- **THEN** the search finds at least one line
- **AND** every matching line names an image tag that starts with `node:26-`

### Requirement: Dependabot keeps the v0.x branch up to date

GitHub reads Dependabot's configuration from the default branch only. So `main`'s `.github/dependabot.yml` SHALL declare the v0.10.x version updates and aim them at the `v0.x` branch. These entries cover version updates only. Security warnings for `v0.x` come from the weekly scan that the `ci` capability defines. It SHALL declare exactly three entries with `target-branch` set to `v0.x`: one each for `npm`, `docker`, and `github-actions`, each with `directory` set to `/`. Every other entry SHALL have no `target-branch`, so it updates `main`.

The scenarios below use `yq` version 4 (the Go implementation). The development shell does not provide it, so a reviewer installs it first.

#### Scenario: The configuration declares the v0.x updates

- **WHEN** a reviewer runs `yq '.updates[] | select(."target-branch" == "v0.x") | ."package-ecosystem"' .github/dependabot.yml`
- **THEN** the output is exactly three lines: `npm`, `docker`, and `github-actions`

#### Scenario: Every v0.x entry checks the repo root

- **WHEN** a reviewer runs `yq '.updates[] | select(."target-branch" == "v0.x") | .directory' .github/dependabot.yml`
- **THEN** every line of the output is `/`

#### Scenario: A v0.10.x version update targets v0.x

- **WHEN** Dependabot opens a version update pull request that changes the root `package.json` or `package-lock.json`
- **THEN** that pull request's base branch is `v0.x`

### Requirement: mise provides the pinned toolchain

A contributor with mise SHALL get these tools, after running `mise trust` and `mise install` once at the repo root:

- Rust at a stable release, with `rustfmt` and `clippy`;
- Node at the major version `.nvmrc` names;
- pnpm at major version 12;
- `just`;
- `gh`, at 2.102.0 or later;
- the `openspec` CLI;
- the `lets` command from Let's Spec.

#### Scenario: A contributor checks tool versions through mise

- **WHEN** a contributor runs `mise exec -- sh -c 'rustc --version; node --version; pnpm --version; just --version; gh --version; openspec --version; lets --version'` at the repo root
- **THEN** `rustc --version` output starts with `rustc 1.` and contains neither `beta` nor `nightly`
- **AND** `node --version` output starts with `v26.`
- **AND** `pnpm --version` output starts with `12.`
- **AND** the `just`, `gh`, `openspec` and `lets` commands each exit with status 0

#### Scenario: Formatting and lint tools come with Rust

- **WHEN** a contributor runs `mise exec -- sh -c 'cargo fmt --version && cargo clippy --version'` at the repo root
- **THEN** both commands exit with status 0

### Requirement: mise takes the Node version from .nvmrc

`mise.toml` SHALL NOT name a Node version. mise SHALL read the Node version from `.nvmrc`, whatever the contributor's own mise settings say about version files.

#### Scenario: A contributor has no mise settings of their own

- **WHEN** a contributor whose global mise configuration is empty runs `mise ls --current node` at the repo root
- **THEN** the output names `.nvmrc` as the source of the Node version

#### Scenario: .nvmrc changes

- **WHEN** a tester changes `.nvmrc` to `24`, then runs `mise install` and `mise exec -- node --version` at the repo root
- **THEN** the output starts with `v24.`

### Requirement: Rust on the mise route follows rustup's stable channel

On the mise route, the Rust release SHALL be the newest stable release rustup knows when the contributor installs or updates it. The mise route SHALL NOT pin an exact Rust release.

#### Scenario: mise uses rustup's stable release

- **WHEN** a contributor runs `mise exec -- rustc --version` and `mise exec -- rustup run stable rustc --version` at the repo root
- **THEN** both commands print the same release

### Requirement: The mise lock pins every other tool's exact release

`mise.lock` SHALL be committed. It SHALL record one exact release for each tool `mise.toml` names, except Rust. For each tool mise downloads as a file, the lock SHALL record a download address and checksum for at least `linux-x64`, `linux-arm64` and `macos-arm64`. `mise install` SHALL install the releases the lock records, and SHALL NOT change the lock.

#### Scenario: The lock is up to date

- **WHEN** a reviewer runs `mise lock` on a clean checkout, then `git diff --exit-code mise.lock`
- **THEN** the `git diff` command exits with status 0

#### Scenario: Installing leaves the lock unchanged

- **WHEN** a contributor runs `mise install` on a clean checkout, then `git status --porcelain mise.lock`
- **THEN** the output is empty

#### Scenario: The lock covers the supported systems

- **WHEN** a reviewer reads the `[[tools.node]]`, `[[tools.pnpm]]`, `[[tools.just]]`, `[[tools.gh]]` and `[[tools."github:sidereal-io/lets-spec"]]` entries in `mise.lock`
- **THEN** each entry has a `platforms.linux-x64`, a `platforms.linux-arm64` and a `platforms.macos-arm64` table
- **AND** each of those tables has a `url` and a `checksum`

#### Scenario: The lock pins Let's Spec at one release

- **WHEN** a reviewer reads the `[[tools."github:sidereal-io/lets-spec"]]` entry in `mise.lock`
- **THEN** its `version` is `0.8.0`

### Requirement: Both routes use one Let's Spec release

The Nix shell and mise SHALL provide the same Let's Spec release and the same `openspec` CLI version. The flake input for Let's Spec SHALL name a release tag, not a branch. The committed skills, schemas and rules block SHALL be the ones that release's `lets sync` writes.

#### Scenario: The flake input names a tag

- **WHEN** a reviewer runs `nix flake metadata --json` at the repo root
- **THEN** the `lets-spec` input's `original` entry has `ref` set to `v0.8.0`

#### Scenario: The two routes report the same versions

- **WHEN** a tester runs `lets --version` and `openspec --version`, once through `nix develop --command` and once through `mise exec --`, at the repo root
- **THEN** the two `lets --version` outputs are identical
- **AND** the two `openspec --version` outputs are identical

#### Scenario: The committed copies match the release

- **WHEN** a reviewer runs `mise exec -- lets sync --check` on a clean checkout
- **THEN** the command exits with status 0

### Requirement: Entering the repo with mise refreshes the skills

For a contributor who has run `mise activate` in their shell and trusted the repo's `mise.toml`, entering the repo directory SHALL run `lets sync`. When the committed copies already match the release, entering SHALL print nothing from `lets sync` and change no file.

#### Scenario: A Let's Spec skill was deleted

- **WHEN** a contributor with mise activated deletes `.agents/skills/lets-plan/SKILL.md`, leaves the repo directory, and enters it again
- **THEN** `.agents/skills/lets-plan/SKILL.md` exists
- **AND** `git status --porcelain` prints nothing

#### Scenario: Nothing changed

- **WHEN** a contributor with mise activated enters the repo directory on a clean checkout
- **THEN** `git status --porcelain` prints nothing
- **AND** `lets sync` writes nothing to standard output or standard error

### Requirement: mise sets the Let's Spec flow and keeps the OpenSpec CLI offline

`mise.toml` SHALL set `LETS_SPEC_FLOW` to `standard` and `OPENSPEC_NO_UPDATE_CHECK` to `1`, so no `openspec` command run through mise checks the npm registry for a newer version.

#### Scenario: A contributor reads the environment through mise

- **WHEN** a contributor runs `mise exec -- sh -c 'printenv LETS_SPEC_FLOW; printenv OPENSPEC_NO_UPDATE_CHECK'` at the repo root
- **THEN** the output is two lines: `standard`, then `1`

### Requirement: Contributors can add their own mise settings

mise SHALL apply the settings in a file named `mise.local.toml` at the repo root when it exists, over those in `mise.toml`. Git SHALL ignore `mise.local.toml`.

#### Scenario: A contributor adds a personal variable

- **WHEN** a contributor writes `[env]` and `SIDEREAL_TEST_VAR = "1"` in `mise.local.toml`, then runs `mise exec -- printenv SIDEREAL_TEST_VAR` at the repo root
- **THEN** the output is `1`

#### Scenario: The personal file stays out of git

- **WHEN** a contributor creates `mise.local.toml` and runs `git status --porcelain`
- **THEN** the output does not list `mise.local.toml`

### Requirement: CI checks the mise route on every pull request that changes it

A CI job, the mise check, SHALL run on every pull request that changes one of these files:

- `mise.toml` or `mise.lock`;
- any file under `.mise/`, which holds the npm dependency lock;
- `server/rust-toolchain.toml` or `.nvmrc`;
- the workflow file that defines the mise check.

The mise check SHALL install the tools from `mise.lock`. It SHALL fail when an install fails.

#### Scenario: A pull request breaks the lock

- **WHEN** a pull request changes the `checksum` of the `linux-x64` entry for `just` in `mise.lock` to a wrong value
- **THEN** the mise check on that pull request fails

#### Scenario: A pull request does not touch the mise route

- **WHEN** a pull request changes none of the files in this requirement's list
- **THEN** GitHub does not run the mise check on that pull request

### Requirement: The mise check fails when the Rust channels differ

The mise check SHALL fail when the Rust channel in `mise.toml` differs from the `channel` in `server/rust-toolchain.toml`. Its log SHALL name both files.

#### Scenario: A pull request changes one channel only

- **WHEN** a pull request changes `channel` in `server/rust-toolchain.toml` to `beta`, and leaves `mise.toml` unchanged
- **THEN** the mise check on that pull request fails
- **AND** its log names `mise.toml` and `server/rust-toolchain.toml`

#### Scenario: The channels agree

- **WHEN** a pull request leaves both files naming `stable`, and the install succeeds
- **THEN** the mise check on that pull request passes

### Requirement: The mise check reports the tool versions

After the install, the mise check SHALL run `rustc --version`, `node --version`, `pnpm --version`, `just --version`, `gh --version`, `openspec --version` and `lets --version` through mise. A reviewer SHALL be able to read their output in the job log. The mise check SHALL fail when any of the seven commands fails.

#### Scenario: A reviewer checks what a pin update changed

- **WHEN** the mise check passes on a pull request
- **THEN** the job log contains the output of each of the seven commands

#### Scenario: A pin update breaks a tool

- **WHEN** `openspec --version` exits with a non-zero status through mise, and the other six commands succeed
- **THEN** the mise check fails

### Requirement: The mise check runs without secrets

The mise check SHALL use no repository secret. Its workflow SHALL grant the job read-only access to repository contents and no other permission.

#### Scenario: A reviewer inspects the workflow permissions

- **WHEN** a reviewer reads the mise check workflow file
- **THEN** its `permissions` block grants only `contents: read`
- **AND** the file contains no reference to `secrets.`
