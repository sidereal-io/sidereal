## MODIFIED Requirements

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
- **THEN** every file under `.agents/skills/peer-code-review`, `.agents/skills/grill-me`, `.agents/skills/choose-an-adversary` and `.agents/skills/discovery` is unchanged, byte for byte

#### Scenario: Running the command twice changes nothing

- **WHEN** a contributor runs `just skills` twice in a row
- **THEN** no file under `.agents/skills` differs between the two results

### Requirement: Every supported agent sees the same skills

Claude Code SHALL see the same skill set as agents that read `.agents/skills`. That set SHALL include both the generated and the authored skills. The repo SHALL keep one copy of each skill.

#### Scenario: Claude sees the generated and authored skills

- **WHEN** a contributor runs `just skills`
- **THEN** `.claude/skills/openspec-propose/SKILL.md` and `.claude/skills/peer-code-review/SKILL.md` both exist
- **AND** `.claude/skills` resolves to the same directory as `.agents/skills`
