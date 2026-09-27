## ADDED Requirements

### Requirement: The skills command gives the same skills on every machine

Running `just skills` SHALL generate the OpenSpec skills from settings the repo defines. The result SHALL NOT depend on the contributor's global OpenSpec config, or on generated files left from an earlier run. The command SHALL generate these 8 workflows, as skills only: `propose`, `explore`, `continue`, `apply`, `update`, `sync`, `archive` and `verify`. The generated skills SHALL use only the generic `/openspec-<name>` form to name other skills.

#### Scenario: Two machines with different global configs agree

- **WHEN** a tester runs `just skills` in two clones of one commit
- **AND** the two runs use different global OpenSpec configs, for example one with the `core` profile and one with `delivery` set to `both`
- **THEN** `diff -r .agents/skills` between the two clones reports no difference

#### Scenario: The expected workflows are generated

- **WHEN** a contributor runs `just skills`
- **THEN** `.agents/skills` contains exactly these generated folders: `openspec-propose`, `openspec-explore`, `openspec-continue-change`, `openspec-apply-change`, `openspec-update-change`, `openspec-sync-specs`, `openspec-archive-change` and `openspec-verify-change`
- **AND** no command files for any agent appear in the repo

#### Scenario: A stale Codex marker is left in the clone

- **WHEN** `.agents/skills/.openspec-target` contains `codex`, and a contributor runs `just skills`
- **THEN** no file under `.agents/skills` contains the text `$openspec-`
- **AND** the generated skills match those from a clone that never had the marker

#### Scenario: A workflow is removed from the repo settings

- **WHEN** a maintainer removes `verify` from the workflow list and runs `just skills`
- **THEN** `.agents/skills/openspec-verify-change` no longer exists

### Requirement: The skills command leaves the contributor's own config alone

`just skills` SHALL NOT read or change the contributor's global OpenSpec config. It SHALL redirect the CLI to the repo's settings for its own run only, so the contributor's shell keeps its environment. It SHALL run the CLI with usage telemetry turned off.

#### Scenario: The global config file is unchanged

- **WHEN** a contributor records a checksum of `~/.config/openspec/config.json`, then runs `just skills`
- **THEN** the checksum of that file is unchanged

#### Scenario: The shell environment is unchanged

- **WHEN** a contributor runs `just skills` in an interactive shell
- **THEN** `XDG_CONFIG_HOME` and `OPENSPEC_TELEMETRY` in that shell keep the values they had before the run

#### Scenario: No telemetry ID is recorded

- **WHEN** a contributor runs `just skills`
- **THEN** `.config/openspec/config.json` has no `telemetry` key

### Requirement: The skills command uses the pinned CLI version

The repo SHALL pin one `openspec` CLI version for generating skills. `just skills` SHALL stop before changing any file when `openspec --version` reports a different version. Its error message SHALL name both versions and the command that installs the pinned one. The Nix development shell SHALL provide the pinned version.

#### Scenario: A contributor has a different CLI version

- **WHEN** a contributor whose `openspec --version` differs from the pinned version runs `just skills`
- **THEN** the command exits with a non-zero status
- **AND** its error message names the installed version, the pinned version, and `npm install -g @fission-ai/openspec@<pinned version>`
- **AND** `git status --porcelain` shows the same output as before the run

#### Scenario: Generated skills record the pinned version

- **WHEN** a contributor with the pinned CLI runs `just skills`
- **THEN** every generated `SKILL.md` has `generatedBy` equal to the pinned version

#### Scenario: The Nix shell matches the pin

- **WHEN** a contributor runs `nix develop --command just skills`
- **THEN** the command exits with status 0

### Requirement: Git tracks only authored skills

Git SHALL track the skills people write, and SHALL NOT track generated skills, the `.openspec-target` marker, or the generated config. The prefix `openspec-` SHALL be reserved for generated skills. `just skills` SHALL NOT change any skill whose folder name lacks that prefix.

#### Scenario: Generating skills leaves git clean

- **WHEN** a contributor runs `just skills` on a clean checkout
- **THEN** `git status --porcelain` prints nothing

#### Scenario: No generated skill is tracked

- **WHEN** a reviewer runs `git ls-files .agents/skills`
- **THEN** no path in the output starts with `.agents/skills/openspec-`
- **AND** the output does not include `.agents/skills/.openspec-target`

#### Scenario: Authored skills survive generation

- **WHEN** a contributor runs `just skills`
- **THEN** every file under `.agents/skills/critique`, `.agents/skills/grill-me`, `.agents/skills/choose-an-adversary` and `.agents/skills/discovery` is unchanged, byte for byte

#### Scenario: Running the command twice changes nothing

- **WHEN** a contributor runs `just skills` twice in a row
- **THEN** no file under `.agents/skills` differs between the two results

### Requirement: Every supported agent sees the same skills

Claude Code SHALL see the same skill set as agents that read `.agents/skills`. That set SHALL include both the generated and the authored skills. The repo SHALL keep one copy of each skill.

#### Scenario: Claude sees the generated and authored skills

- **WHEN** a contributor runs `just skills`
- **THEN** `.claude/skills/openspec-propose/SKILL.md` and `.claude/skills/critique/SKILL.md` both exist
- **AND** `.claude/skills` resolves to the same directory as `.agents/skills`

### Requirement: Skills work without Nix

A contributor without Nix SHALL be able to generate the skills with `just` and the pinned `openspec` CLI installed by hand. The result SHALL match the result inside the Nix shell.

#### Scenario: A contributor without Nix generates skills

- **WHEN** a contributor without Nix installs `just` and the pinned `openspec` version through npm, then runs `just skills`
- **THEN** the command exits with status 0
- **AND** `diff -r .agents/skills` against a run inside the Nix shell reports no difference
