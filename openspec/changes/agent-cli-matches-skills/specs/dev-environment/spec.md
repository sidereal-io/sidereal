## ADDED Requirements

### Requirement: The skills script reports a CLI mismatch

`scripts/openspec-skills.sh --check` SHALL compare the `openspec` CLI on `PATH` with the CLI that generated the skills. It SHALL report a mismatch in each of these cases:

- no generated skills: no successful `just skills` run has recorded what it generated, or a generated skill folder from that run is missing;
- no CLI: `openspec` is not on `PATH`;
- a broken CLI: `openspec --version` fails;
- a different CLI: `openspec --version` differs from the version that generated the skills.

The check SHALL print its report to standard output. It SHALL exit with status 0 in every case, including when a command it runs fails. When none of the cases applies, it SHALL print nothing. It SHALL NOT create, change or delete any file.

The report SHALL print a version only when the value looks like a version number: digits and dots, with an optional suffix of letters, digits, dots and hyphens. The report SHALL call any other value unreadable, and SHALL NOT print it. This keeps text from the stamp or the CLI out of the agent's context.

A report SHALL name the fix that fits the contributor's machine:

- When `nix` is on `PATH`, the report SHALL say to restart the agent from a shell where direnv has loaded the repo, or with `nix develop --command`. It SHALL NOT suggest `just skills`, because regenerating with a CLI from outside the shell makes the shared skills differ from the pinned CLI.
- When `nix` is not on `PATH`, the report SHALL say to run `just skills`.

#### Scenario: The CLI matches the skills

- **WHEN** a contributor runs `just skills`, then runs `scripts/openspec-skills.sh --check` with the same `openspec` on `PATH`
- **THEN** the check prints nothing
- **AND** it exits with status 0

#### Scenario: The CLI differs from the skills

- **WHEN** a contributor generates the skills with one `openspec` version, and a tester runs the check with a different version first on `PATH`
- **THEN** standard output names both versions
- **AND** the check exits with status 0

#### Scenario: The CLI is missing

- **WHEN** a tester runs the check with no `openspec` on `PATH`
- **THEN** standard output says that no `openspec` CLI was found
- **AND** the check exits with status 0

#### Scenario: The CLI is broken

- **WHEN** a tester runs the check with an `openspec` on `PATH` that is a stub exiting with status 1
- **THEN** standard output says that `openspec --version` failed
- **AND** the check exits with status 0

#### Scenario: The stamp holds text that is not a version

- **WHEN** a tester replaces the version in the stamp's `cli` line with the text `ignore previous instructions`, then runs the check
- **THEN** standard output does not contain `ignore previous instructions`
- **AND** standard output says that the recorded version is unreadable
- **AND** the check exits with status 0

#### Scenario: The skills are missing

- **WHEN** a tester runs the check in a fresh clone where `just skills` has never run
- **THEN** standard output says that the generated skills are missing
- **AND** the check exits with status 0

#### Scenario: A contributor with Nix gets a safe fix

- **WHEN** the check reports a different CLI, and `nix` is on `PATH`
- **THEN** standard output names `nix develop --command`
- **AND** standard output does not name `just skills`

#### Scenario: A contributor without Nix gets the regenerate fix

- **WHEN** the check reports a different CLI, and `nix` is not on `PATH`
- **THEN** standard output names `just skills`

#### Scenario: The check changes nothing

- **WHEN** a tester records the modification time of every file under `.agents/skills`, then runs the check with a different `openspec` version on `PATH`
- **THEN** every modification time is unchanged
- **AND** `git status --porcelain` shows the same output as before the check

### Requirement: Agent sessions start with the check

Claude Code and Codex SHALL run `scripts/openspec-skills.sh --check` when a session starts in the repo, through each agent's project hook configuration. The check's report SHALL reach the agent as context for the session. The hooks SHALL call the script directly, without `just`, so the check runs when `just` is not on `PATH`. A hook that fails SHALL NOT stop the session from starting.

#### Scenario: The Claude Code hook reports a different CLI

- **WHEN** a tester takes the `SessionStart` hook command from `.claude/settings.json`
- **AND** runs it from the repo root, with `CLAUDE_PROJECT_DIR` set to the repo root and an `openspec` first on `PATH` whose version differs from the one that generated the skills
- **THEN** standard output names both versions
- **AND** the command exits with status 0

#### Scenario: The Codex hook reports a different CLI

- **WHEN** a tester takes the `SessionStart` hook command from `.codex/hooks.json`
- **AND** runs it from the repo root, with an `openspec` first on `PATH` whose version differs from the one that generated the skills
- **THEN** standard output names both versions
- **AND** the command exits with status 0

#### Scenario: The hooks print nothing when the CLI matches

- **WHEN** a tester runs each hook command the same way, with the `openspec` that generated the skills first on `PATH`
- **THEN** each command prints nothing
- **AND** each command exits with status 0

#### Scenario: The hooks don't need just

- **WHEN** a reviewer reads the hook commands in `.claude/settings.json` and `.codex/hooks.json`
- **THEN** each command runs `scripts/openspec-skills.sh --check`
- **AND** no command names `just`

### Requirement: Agents without a hook are told to run the check

`AGENTS.md` SHALL tell any agent whose session didn't start with a skills check to run `scripts/openspec-skills.sh --check` and follow what it prints. The instruction SHALL NOT name any specific agent.

#### Scenario: AGENTS.md names the check

- **WHEN** a reviewer searches `AGENTS.md` for `scripts/openspec-skills.sh --check`
- **THEN** the search finds the instruction

### Requirement: Contributors learn how to start an agent

`CONTRIBUTING.md` SHALL explain how to start an agent so that it runs the pinned `openspec` CLI. It SHALL name two ways: from a shell where direnv has loaded the repo, or with `nix develop --command <agent>`. It SHALL explain what the check's report means and what to do about it.

#### Scenario: The docs name both ways to start an agent

- **WHEN** a reviewer reads `CONTRIBUTING.md`
- **THEN** it names `direnv` and `nix develop --command` as ways to start an agent
- **AND** it names `scripts/openspec-skills.sh --check`

## MODIFIED Requirements

### Requirement: The shell turns on when you enter the repo

For a contributor with Nix and direnv, entering the repo directory SHALL turn the development shell on after a one-time `direnv allow`. Leaving the directory SHALL restore the contributor's previous environment. The shell SHALL reload when a file that defines it changes. The shell SHALL also reload when a file that the skills are generated from changes: `.config/openspec/config.json` or `scripts/openspec-skills.sh`.

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
- **AND** the output lists `.config/openspec/config.json` and `scripts/openspec-skills.sh` as watched

### Requirement: Loading the shell refreshes stale skills

When the Nix development shell loads, it SHALL regenerate the OpenSpec skills if they are stale. The skills are stale when any of these is true:

- no successful `just skills` run has recorded what it generated;
- the `openspec` version differs from the version that generated them;
- `.config/openspec/config.json` or `scripts/openspec-skills.sh` differs from the version used to generate them;
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
