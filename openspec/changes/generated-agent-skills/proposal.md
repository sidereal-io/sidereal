Origin: [#274](https://github.com/sidereal-io/sidereal/issues/274) (`generated-agent-skills`).

## Why

The OpenSpec skills in `.agents/skills` differ from machine to machine, and git tracks them anyway:

- **The global config decides the output.** `openspec init` reads the workflows and the delivery mode only from each person's `~/.config/openspec/config.json`. Two contributors get different skills from the same commit.
- **A committed marker adds Codex hints.** `.agents/skills/.openspec-target` says `codex`. While it exists, `openspec init --tools agents,claude` still renders Codex text such as `$openspec-apply-change (Codex) or /openspec-apply-change (other agents)`.
- **Generated files sit in git.** Every CLI upgrade rewrites eight tracked `SKILL.md` files.

Two later stories build on this one: refreshing skills when you enter the shell (#275), and a CI check for skill drift (#276).

## What Changes

- **A new `just skills` recipe.** It calls `scripts/skills.sh`, which generates the OpenSpec skills the same way on every machine:
  1. It checks that `openspec` is on `PATH`. If not, it stops and prints the install command.
  2. It deletes the generated `openspec-*` skills and the `.openspec-target` marker, so leftover files on a machine cannot change the result.
  3. It runs `openspec init --tools agents` with `XDG_CONFIG_HOME` pointed at the repo's `.config` folder and telemetry turned off. Both apply to that one command only.
- **The repo tracks the OpenSpec settings.** `.config/openspec/config.json` sets the custom profile, 8 workflows and skills-only delivery. The CLI reads it through the redirect.
- **Nix stays the only pin for the CLI version.** Contributors without Nix install the latest release.
- **A source comment marks the redirect as temporary.** It links the upstream OpenSpec issues for project-scoped settings ([Fission-AI/OpenSpec#914](https://github.com/Fission-AI/OpenSpec/issues/914) and [#779](https://github.com/Fission-AI/OpenSpec/issues/779)). It says to remove the redirect once the CLI reads those settings from the repo.
- **Git holds only authored skills.** The generated `openspec-*` skills and the marker leave git, and `.gitignore` covers them.
- **`.claude/skills` stays a tracked link to `../.agents/skills`.** OpenSpec 1.13.1 renders identical skills for `agents` and `claude`, so Claude reads the one generated set through the link.
- **Docs explain the new step.** `CONTRIBUTING.md` and `AGENTS.md` describe:
  - running `just skills`, and installing `openspec` without Nix;
  - the `openspec-` prefix, which only generated skills may use;
  - where the skill settings live;
  - how to add another agent.

Nothing here is **BREAKING**. After pulling this change, each contributor runs `just skills` once to get the skills back.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `dev-environment`: adds requirements for generating agent skills. They cover the same output on every machine, the Nix shell as the source of the CLI, and git tracking authored skills and the skill settings.

## Impact

- **New files:**
  - `scripts/skills.sh` — the steps of `just skills`;
  - `.config/openspec/config.json` — the tracked skill settings.
- **Changed files:**
  - `justfile` — adds a one-line `skills` recipe;
  - `.gitignore` — ignores the generated skills and the marker;
  - `CONTRIBUTING.md` and `AGENTS.md`.
- **Removed from git:**
  - the eight `.agents/skills/openspec-*` folders;
  - `.agents/skills/.openspec-target`.
- **Not affected:**
  - the authored skills: `critique`, `grill-me`, `choose-an-adversary` and `discovery`;
  - the `.claude/skills` link;
  - `openspec/config.yaml` — the CLI does not read workflow settings from it;
  - each contributor's own global OpenSpec config;
  - application code in either stack;
  - the Nix shell and CI.
