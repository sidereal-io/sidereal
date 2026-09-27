## Context

See `proposal.md` for why this change exists. The requirements are in `specs/dev-environment/spec.md`. This design records how `just skills` meets them, and why.

Facts about the OpenSpec CLI that shape the approach. Each was checked against 1.13.1, the version the Nix shell pins:

- **Settings come only from the global config.** `openspec init` reads `profile`, `delivery` and `workflows` from `$XDG_CONFIG_HOME/openspec/config.json`, or from `~/.config/openspec/config.json` when that variable is unset. No flag or environment variable sets the workflows or the delivery mode. `openspec/config.yaml` holds only the schema, context and rules.
- **A marker picks the renderer for `.agents/skills`.** Codex and the generic `agents` tool share that folder. `.agents/skills/.openspec-target` names which of the two renders it. An existing `codex` marker wins, even when `--tools` lists only `agents` and `claude`.
- **`agents` and `claude` render identical skills.** With `delivery: skills`, the two outputs match byte for byte. Only the marker differs.
- **`init` leaves other folders alone.** Running `openspec init --tools agents` does not touch authored skill folders or a `.claude/skills` link.
- **Telemetry can be turned off per command.** With `OPENSPEC_TELEMETRY=0`, the CLI did not write to the config file it read.

The repo today tracks the 8 generated skills, a `codex` marker, and the `.claude/skills -> ../.agents/skills` link.

## Goals / Non-Goals

**Goals:**

- The output of `just skills` depends only on the pinned CLI version and the settings in the `justfile`.
- One file, the `justfile`, holds every skill setting: the CLI version, the agent list and the workflow list.

**Non-Goals:**

- Running `just skills` when you enter the shell, and silencing the CLI's update check (#275).
- A CI check for skill drift (#276).
- Native Windows. The repo already relies on symlinks and `sh`.
- A separate skill set for an agent that needs different skill text. No current agent does.

## Decisions

### D1. Redirect `XDG_CONFIG_HOME` for one command

`just skills` sets `XDG_CONFIG_HOME` and `OPENSPEC_TELEMETRY=0` as a prefix on the `openspec init` command. It does not export either variable.

- **Why:** the CLI accepts these settings from no other source. A prefix affects only that process, so the contributor's shell, editor and real config stay untouched.
- **Alternative — `--profile core`:** it gives 6 fixed workflows, without `continue` and `verify`. The delivery mode would still come from the global config.
- **Alternative — each contributor edits their global config:** that is today's approach, and it causes the drift.
- **Exit plan:** a comment above the command links [Fission-AI/OpenSpec#914](https://github.com/Fission-AI/OpenSpec/issues/914) and [#779](https://github.com/Fission-AI/OpenSpec/issues/779). It says to drop the redirect once the CLI reads project-scoped `profile`, `delivery` and `workflows`.

### D2. Write the config at run time into `.config/openspec/`

`just skills` writes `.config/openspec/config.json` on every run, from values in the `justfile`. Git ignores `/.config/openspec/` only, not all of `.config/`.

- **Why:** the workflow list sits beside the agent list and the version pin, so one file holds every skill setting. A fresh file on every run also means nothing the CLI writes can carry over.
- **Alternative — a tracked config file:** it would be easier to diff. It also adds a second file named `config` in the repo, next to `openspec/config.yaml`, which readers confuse.
- **Alternative — `.workspace/`:** that folder is kept for agents' scratch files.

### D3. Delete generated files before `init`

Before `init`, `just skills` deletes `.agents/skills/openspec-*` and `.agents/skills/.openspec-target`. It deletes nothing else.

- **Why:** a stale `codex` marker, or leftover skills, change what `init` renders. Deleting them first means every run starts from the same state. It also removes a workflow's skill after the workflow leaves the list.
- **Alternative — delete only the marker:** `init` also infers the renderer from leftover skill text, so the skills must go too.
- **Consequence:** `openspec-` becomes a reserved prefix. An authored skill with that prefix would be deleted. The docs say so.

### D4. Generate for `agents` only, and keep the `.claude/skills` link

`just skills` runs `openspec init --tools agents`. `.claude/skills` stays a tracked link to `../.agents/skills`.

- **Why:** the `claude` output is identical, so generating it would make a second copy of the same files. The link already gives Claude the authored skills, so no linking step is needed.
- **Alternative — `--tools agents,claude` with a real `.claude/skills` folder:** it needs a link or a copy for each authored skill, plus more ignore rules. A copied skill can be edited by mistake and then silently overwritten.
- **Adding an agent:** an agent that reads `.agents/skills` needs nothing. An agent that reads only its own folder gets a folder link, like Claude. An agent that needs different skill text needs its own generated set, which is a future change.

### D5. Pin the CLI version in the `justfile` and check it

The `justfile` holds `openspec_version := "1.13.1"`. `just skills` compares it with `openspec --version` first, and stops on a mismatch. The error names both versions and `npm install -g @fission-ai/openspec@<version>`.

- **Why:** the Nix shell pins the CLI through `flake.lock`, but contributors without Nix have no pin at all. The check keeps both routes on one version. It also catches a `flake.lock` bump that moves the Nix CLI, because `just skills` then fails inside the shell until someone updates the pin.
- **Alternative — an npm devDependency:** one lock file would pin it for everyone. It would add v2 tooling to the v0.10.x `package.json`, which the cutover replaces, and put a second `openspec` on `PATH` in the Nix shell.
- **Alternative — `npx @fission-ai/openspec@<version>`:** nothing to install. The first run needs the network, which conflicts with running offline on shell entry (#275).
- **Alternative — a warning instead of an error:** it is easy to miss, and the skills would still differ.

### D6. Write the recipe as one bash script

The recipe uses a `#!/usr/bin/env bash` shebang with `set -euo pipefail`.

- **Why:** the steps depend on each other. A failed version check must stop the run before any file is deleted. In a shebang recipe, `just` runs all the lines in one shell, so an early exit stops everything after it.

## Risks / Trade-offs

- **A future CLI renders `claude` skills differently** → Claude would read the generic text, which still works. The spec scenario "Claude sees the generated and authored skills" and #276's drift check are where to catch it. Move Claude to its own folder if that happens.
- **A `flake.lock` bump breaks `just skills` in the Nix shell** → this is intended. The error names the new version, and the fix is a one-line pin change. Scheduled flake updates (#287) must bump the pin too.
- **Someone runs `openspec init` or `openspec update` by hand** → it renders with their global settings into ignored files. The next `just skills` restores the repo's settings. `AGENTS.md` tells agents to use `just skills`.
- **An authored skill is named `openspec-something`** → `just skills` deletes it. The docs reserve the prefix.

## Migration Plan

1. Merge the change. When a contributor pulls it, git deletes the tracked generated skills and the marker from their working tree.
2. Each contributor runs `just skills` once. Without the pinned CLI, the command tells them what to install.
3. Until they run it, agents in that clone see only the authored skills.

**Rollback:** revert the merge commit. The generated skills and the marker return to git.
