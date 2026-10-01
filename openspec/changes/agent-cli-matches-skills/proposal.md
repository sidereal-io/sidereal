Origin: [#306](https://github.com/sidereal-io/sidereal/issues/306) (`agent-cli-matches-skills`).

## Why

An agent can follow skills that its own `openspec` CLI doesn't support, and nothing tells it so:

- **Every agent reads the same skills.** The generated skills sit on disk in `.agents/skills`, so all agents in the repo share one copy.
- **Each agent runs its own CLI.** An agent runs whatever `openspec` is first on its `PATH`. An agent started outside the loaded Nix shell runs a global CLI instead of the pinned one.
- **The versions can differ.** On one machine, the global CLI was 1.12.0, but the skills came from the pinned 1.13.1. The skills name commands and flags, so the agent may follow steps its CLI can't run.

Since [#275](https://github.com/sidereal-io/sidereal/issues/275), a stamp records the CLI version that generated the skills. This change compares that version with the agent's CLI when a session starts.

## What Changes

- **The skills script checks for a mismatch.** `scripts/openspec-skills.sh --check` compares the agent's `openspec --version` with the version in the stamp. It reports these problems:
  - no generated skills;
  - no `openspec` CLI on `PATH`;
  - a CLI that fails to report its version;
  - a CLI version that differs from the stamp.

  The check writes no files, prints its warning to standard output, and always exits 0. It prints nothing when the versions match.
- **The warning names a safe fix.** With Nix installed, it says to restart the agent from the Nix shell. It does not suggest `just skills` in that case: regenerating with the global CLI would make the shared skills differ from the pinned CLI. Without Nix, it says to run `just skills`.
- **Claude Code and Codex run the check when a session starts.** A `SessionStart` hook in `.claude/settings.json` and in `.codex/hooks.json` calls the script directly. It does not go through `just`, because `just` may be missing outside the shell.
- **Other agents get one instruction.** `AGENTS.md` gains one line: if your session didn't start with a skills check, run the check and follow what it prints.
- **The script gets a clearer name.** `scripts/skills.sh` becomes `scripts/openspec-skills.sh`. `git mv` keeps its history. The `justfile`, `.envrc`, `CONTRIBUTING.md` and the `dev-environment` spec follow the new name. Archived changes keep the old name.
- **Docs explain how to start an agent.** `CONTRIBUTING.md` gains a section on starting an agent from a shell where direnv has loaded the repo, or with `nix develop --command <agent>`. It also explains the check's warning. The existing warning about untrusted branches grows to cover agents: the hooks run the checked-out branch's script when a session starts.

`just skills` keeps its name. Nothing here is **BREAKING**.

**Effects contributors will notice once:**

- Changing `.envrc` makes direnv ask for approval again. Contributors with direnv run `direnv allow` once.
- Changing the script's contents makes the stamp stale. The shell regenerates the skills once on every machine.
- Codex asks the user to trust the new project hook before it runs.

**Out of scope:**

- making agents load the Nix shell on their own;
- changing the generated skill text;
- a hook for Antigravity, which has no session-start event;
- checking again during a session, for example after a `git pull` that changes the skill settings.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `dev-environment`: adds requirements for the mismatch check and for the hooks that run it when a session starts. The requirements that name `scripts/skills.sh` change to the new name: "The shell turns on when you enter the repo" and "Loading the shell refreshes stale skills".

## Impact

- **Changed files:**
  - `scripts/skills.sh` — renamed to `scripts/openspec-skills.sh`, and gains `--check`;
  - `justfile` — the `skills` and `enter` recipes call the new name;
  - `.envrc` — watches the new name;
  - `.claude/settings.json` — adds the `SessionStart` hook;
  - `.codex/hooks.json` — new file with the `SessionStart` hook;
  - `AGENTS.md` — adds the check instruction;
  - `CONTRIBUTING.md` — the new name, and a section on starting an agent.
- **Not affected:**
  - what `just skills` generates;
  - the authored skills and the `.claude/skills` link;
  - application code in either stack;
  - CI. The hooks run only in agent sessions.
