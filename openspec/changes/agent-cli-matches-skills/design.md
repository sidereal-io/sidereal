## Context

See `proposal.md` (Why) for the problem, and the delta spec for the required behavior.

These facts about the current repo shape the design:

- `scripts/skills.sh` generates the skills and writes `.agents/skills/.openspec-stamp`. The stamp holds the CLI version (`cli 1.13.1`), two content hashes, and one `skill <name>` line per generated folder. The script's `current_inputs` and `skills_are_fresh` functions already read and compare these lines.
- The Nix shell runs `just enter` when it loads. `enter` regenerates the skills only when they are stale.
- Agents read their skills when a session starts. A fix found later in the session can't change what the agent already read.
- Agent commands don't run at a shell prompt, so direnv doesn't reload the environment during an agent session.
- Running `openspec --version` takes about 0.3 seconds. Hashing the input files takes almost no time.

How the pieces fit together:

```
  session starts
       |
       v
  +----------------------------+     +------------------------------+
  | .claude/settings.json      |     | .codex/hooks.json            |
  |   SessionStart hook        |     |   SessionStart hook          |
  +-------------+--------------+     +--------------+---------------+
                |                                   |
                +-----------------+-----------------+
                                  v
                 scripts/openspec-skills.sh --check
                   reads .agents/skills/.openspec-stamp
                   runs  openspec --version
                                  |
              +-------------------+-------------------+
              v                                       v
        match: prints nothing             mismatch: prints a report
                                          -> agent context
```

Agents without a hook follow the `AGENTS.md` instruction and run the same command.

## Goals / Non-Goals

**Goals:**

- Keep one copy of the logic that reads the stamp.
- Make the check work outside the Nix shell, where `just` may be missing.
- Make sure no report ever suggests a fix that overwrites the shared skills with the wrong CLI.

**Non-Goals:**

- Reporting stale skill settings. If `.config/openspec/config.json` or the script changed since the last run, the check stays silent. The shell refresh already covers that case.
- Fixing the mismatch automatically. The check only reports.
- Supporting agents other than Claude Code and Codex through hooks.

## Decisions

### D1. Session-start hooks, not an instruction alone

Claude Code and Codex run the check through a `SessionStart` hook.

- **Why:** a hook runs every time, whatever the agent decides. It also runs in the agent's own environment, so `openspec` resolves exactly as it does for the agent's commands.
- **Alternative: an `AGENTS.md` instruction only.** Every agent can read it, but an agent can skip it. The design keeps it only as the fallback for agents without a hook.
- **Alternative: add the check to `just check`.** The report would arrive after the agent had already done its work.

### D2. `SessionStart`, not `PreToolUse`

- **Why:** a mismatch matters from the moment the agent reads its skills, which happens when the session starts. Every fix needs a new session anyway.
- **Alternative: `PreToolUse` on `openspec` commands.** It fires only when the agent runs `openspec`. An agent with no generated skills never runs `openspec`, so it would never get a report. It would also add a check to every shell command.

### D3. The check is a `--check` option in the skills script

- **Why:** the script already parses the stamp. A second script would need its own copy of the stamp format, and the two copies could drift apart.
- **Alternative: a separate `scripts/check-skills.sh`.** Its only benefit is avoiding the one-time regeneration that any edit to the skills script causes.

`--check` returns before the script's cleanup step, so it never reaches code that deletes or writes files. It also runs before the existing "CLI missing" exit, because a missing CLI is a case the check reports, not an error.

### D4. The check compares only the CLI version and the skill folders

The check reads the stamp's `cli` line and its `skill` lines. It ignores the two content hashes.

| Case | How the check detects it |
|---|---|
| No generated skills | No stamp, no `skill` lines, or a listed folder without `SKILL.md` |
| No CLI | `command -v openspec` fails |
| A broken CLI | `openspec --version` exits non-zero |
| A different CLI | `openspec --version` differs from the stamp's `cli` line |

The check reports every case that applies, one line each, followed by one line with the fix. If the CLI is missing or broken, it skips the version comparison.

The script runs under `set -euo pipefail`, which would end it on the first failed command. The `--check` path therefore guards every command it runs, so a failure becomes a reported case and the check still exits 0.

Before printing a version, the check matches it against a version pattern: digits and dots, with an optional suffix. A value that doesn't match prints as "unreadable". The check never prints raw text from the stamp or the CLI.

- **Why ignore the hashes:** a hash mismatch means the skill settings changed. Its fix is a shell reload, which is a different problem from the one this change solves (see Non-Goals).

### D5. The fix depends on whether `nix` is on `PATH`

| Machine | Fix the report names |
|---|---|
| `nix` on `PATH` | Restart the agent from a shell where direnv has loaded the repo, or run `nix develop --command <agent>` |
| No `nix` | Run `just skills`, then restart the agent |

- **Why:** with Nix, the stamp normally holds the pinned version. Running `just skills` with a global CLI would replace the shared skills. The next shell load would then replace them again, and the two would keep overwriting each other. Without Nix, the stamp holds the contributor's own CLI version, so regenerating is safe.
- **Alternative: test `IN_NIX_SHELL` or `DIRENV_DIR`.** These show whether the agent is inside the shell. The question that matters is whether the pinned shell is available, and `nix` on `PATH` answers it.

### D6. The report speaks to the agent

The report is written for an agent to read. Its first line names the problem and both versions. Its last line tells the agent to tell the user before it follows any `openspec-*` skill, because an agent can't restart itself.

The check prints the report to standard output and exits 0. Claude Code adds a `SessionStart` hook's standard output to the session's context when the hook exits 0. A non-zero exit would show the text only to the user.

### D7. The hooks call the script directly

Each hook runs the script by its path from the repo root, never through `just`.

- Claude Code: `"$CLAUDE_PROJECT_DIR"/scripts/openspec-skills.sh --check`.
- Codex: the same script. The first task confirms how a Codex project hook finds the repo root.

`just` may be missing outside the Nix shell, which is exactly where the check matters. The script needs only `bash`, `git` and, when installed, `openspec`.

### D8. Rename with `git mv`

`scripts/skills.sh` becomes `scripts/openspec-skills.sh`. The stamp keeps its `script` label, and `current_inputs` hashes the new path. Archived changes keep the old name, because they record past decisions.

## Risks / Trade-offs

- **Codex might not read a project-level `.codex/hooks.json`, or might not pass standard output to the agent.** → The first task tests both on this machine before any hook is written. If either fails, Codex relies on the `AGENTS.md` instruction, and the spec's Codex scenario changes before implementation.
- **A session can start while another window regenerates the skills.** Regeneration deletes the stamp and the skill folders, then writes them again, which takes about a second. A session that starts in that window gets a "skills are missing" report. → Accepted. The report is accurate, because the agent really did start without the skills, and its fix is a restart. An agent started from the terminal that is loading the shell can't hit the window, because direnv finishes before the prompt returns.
- **The stamp is a git-ignored file that anyone with write access can edit.** Its text could carry instructions into the agent's context. → The check prints only version-shaped values (D4). The stamp opens no new route: whoever can edit it can also edit the git-ignored `SKILL.md` files, which the agent reads directly.
- **An agent might ignore the report.** → The report tells the agent to tell the user first. The `AGENTS.md` instruction repeats the rule.
- **A contributor might have Nix installed but not use the shell.** → The report then suggests the shell instead of `just skills`. The contributor can still run `just skills` by hand. Suggesting it automatically would put the shared skills at risk for everyone else using the shell.
- **The hook adds about 0.3 seconds to each session start.** → Accepted. It runs once per session.
- **Codex asks the user to trust the new hook.** → `CONTRIBUTING.md` says to expect the prompt.

## Migration Plan

1. Merge the change.
2. Contributors with direnv run `direnv allow` once, because `.envrc` changed.
3. The next shell load regenerates the skills once, because the script's contents changed.
4. Codex users trust the project hook when Codex asks.

To roll back, revert the merge commit. The old script name returns, and the hooks disappear with it.
