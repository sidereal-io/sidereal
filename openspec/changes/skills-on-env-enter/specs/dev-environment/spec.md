## MODIFIED Requirements

### Requirement: The shell turns on when you enter the repo

For a contributor with Nix and direnv, entering the repo directory SHALL turn the development shell on after a one-time `direnv allow`. Leaving the directory SHALL restore the contributor's previous environment. The shell SHALL reload when a file that defines it changes. The shell SHALL also reload when a file that the skills are generated from changes: `.config/openspec/config.json` or `scripts/skills.sh`.

#### Scenario: A contributor enters and leaves the repo

- **WHEN** a contributor with Nix and direnv has run `direnv allow` once, then changes into the repo directory
- **THEN** `rustc --version` reports the pinned version without any further command
- **AND** after the contributor changes to a directory outside the repo, `rustc --version` reports the same result as before they entered it

#### Scenario: The Rust pin changes while the shell is active

- **WHEN** a tester with the shell loaded changes `channel` in `backend/rust-toolchain.toml` to a different stable version, without leaving the repo directory
- **AND** runs `direnv export bash` in the repo directory
- **THEN** `rustc --version` in that shell reports the new version, not the one from before the edit

#### Scenario: The watch list covers the shell's files

- **WHEN** a tester with the shell loaded runs `direnv status` in the repo directory
- **THEN** the output lists `backend/rust-toolchain.toml` and each file under `nix/` as watched
- **AND** the output lists `.config/openspec/config.json` and `scripts/skills.sh` as watched

## ADDED Requirements

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

### Requirement: Contributors without Nix can opt in to the refresh

A contributor without Nix SHALL be able to get the same refresh by running `just enter` from `.envrc.local`. Without that opt-in, the environment SHALL keep its current behavior for contributors without Nix.

#### Scenario: A contributor without Nix opts in

- **WHEN** a contributor without Nix, with `just` and `openspec` installed, writes `just enter` in `.envrc.local`
- **AND** deletes `.agents/skills/openspec-explore`, then enters the repo directory
- **THEN** `.agents/skills/openspec-explore/SKILL.md` exists
