## Context

See `proposal.md` for why this change exists. The requirements are in `specs/dev-environment/spec.md`. This design records how `just skills` meets them, and why.

Facts about the OpenSpec CLI that shape the approach. Each was checked against 1.13.1, the version the Nix shell provides:

- **Settings come only from the global config.** `openspec init` reads `profile`, `delivery` and `workflows` from `$XDG_CONFIG_HOME/openspec/config.json`, or from `~/.config/openspec/config.json` when that variable is unset. No flag or environment variable sets the workflows or the delivery mode. `openspec/config.yaml` holds only the schema, context and rules.
- **A marker picks the renderer for `.agents/skills`.** Codex and the generic `agents` tool share that folder. `.agents/skills/.openspec-target` names which of the two renders it. An existing `codex` marker wins, even when `--tools` lists only `agents` and `claude`.
- **`agents` and `claude` render identical skills.** With `delivery: skills`, the two outputs match byte for byte. Only the marker differs.
- **`init` leaves other folders alone.** Running `openspec init --tools agents` does not touch authored skill folders or a `.claude/skills` link.
- **`init` does not write to the config it reads.** With `OPENSPEC_TELEMETRY=0`, the config file was byte for byte the same after the run.

The repo today tracks the 8 generated skills, a `codex` marker, and the `.claude/skills -> ../.agents/skills` link.

## Goals / Non-Goals

**Goals:**

- For a given CLI version, the output of `just skills` depends only on settings tracked in the repo.
- One tracked file, `.config/openspec/config.json`, holds the skill settings: profile, delivery and workflows. The Nix lock file holds the CLI version.

**Non-Goals:**

- Running `just skills` when you enter the shell, and silencing the CLI's update check (#275).
- A CI check for skill drift (#276).
- Linting shell scripts with shellcheck.
- Native Windows. The repo already relies on symlinks and `sh`.
- A separate skill set for an agent that needs different skill text. No current agent does.

## Decisions

### D1. Redirect `XDG_CONFIG_HOME` for one command

`just skills` sets `XDG_CONFIG_HOME` to the repo's `.config` folder, and `OPENSPEC_TELEMETRY=0`, as a prefix on the `openspec init` command. It does not export either variable.

- **Why:** the CLI accepts these settings from no other source. A prefix affects only that process, so the contributor's shell, editor and real config stay untouched.
- **Alternative — `--profile core`:** it gives 6 fixed workflows, without `continue` and `verify`. The delivery mode would still come from the global config.
- **Alternative — each contributor edits their global config:** that is today's approach, and it causes the drift.
- **Exit plan:** a comment above the command links [Fission-AI/OpenSpec#914](https://github.com/Fission-AI/OpenSpec/issues/914) and [#779](https://github.com/Fission-AI/OpenSpec/issues/779). It says to drop the redirect once the CLI reads project-scoped `profile`, `delivery` and `workflows`.

### D2. Track the settings in `.config/openspec/config.json`

Git tracks `.config/openspec/config.json`. It sets `profile: custom`, `delivery: skills` and the 8 workflows. The CLI reads it through the D1 redirect, and `just skills` never writes it.

- **Why:** the settings sit in the file the CLI reads, so no code has to build that file. A change to the settings shows as a JSON diff in review. If a later CLI writes to its config, `git status` shows the change rather than hiding it.
- **Alternative — write the file on every run from values in the `justfile`:** it adds settings to the `justfile`, plus code that builds JSON. A fresh file on every run would also hide any write by the CLI.
- **Alternative — `.workspace/`:** that folder is kept for agents' scratch files.
- **Naming:** `.config/openspec/config.json` and `openspec/config.yaml` are different files. `CONTRIBUTING.md` says which settings each one holds.

### D3. Delete generated files before `init`

Before `init`, `just skills` deletes `.agents/skills/openspec-*` and `.agents/skills/.openspec-target`. It deletes nothing else. D7 keeps the deletion inside the repo.

- **Why:** a stale `codex` marker, or leftover skills, change what `init` renders. Deleting them first means every run starts from the same state. It also removes a workflow's skill after the workflow leaves the list.
- **Alternative — delete only the marker:** `init` also infers the renderer from leftover skill text, so the skills must go too.
- **Consequence:** `openspec-` becomes a reserved prefix. An authored skill with that prefix would be deleted. The docs say so.

### D4. Generate for `agents` only, and keep the `.claude/skills` link

`just skills` runs `openspec init --tools agents`. `.claude/skills` stays a tracked link to `../.agents/skills`.

- **Why:** the `claude` output is identical, so generating it would make a second copy of the same files. The link already gives Claude the authored skills, so no linking step is needed.
- **Alternative — `--tools agents,claude` with a real `.claude/skills` folder:** it needs a link or a copy for each authored skill, plus more ignore rules. A copied skill can be edited by mistake and then silently overwritten.
- **Adding an agent:** an agent that reads `.agents/skills` needs nothing. An agent that reads only its own folder gets a folder link, like Claude. An agent that needs different skill text needs its own generated set, which is a future change.

### D5. Let Nix own the CLI version

The Nix lock file is the only pin for the `openspec` version. `just skills` checks only that `openspec` is on `PATH`. If it is missing, the command stops before changing any file, and names `npm install -g @fission-ai/openspec`.

- **Why:** a second pin outside Nix cannot choose the version. It can only detect that Nix moved it. It would also break `just skills` on every `flake.lock` bump, until someone edited the pin by hand.
- **Why check for the CLI at all:** without the check, a missing CLI fails only after the deletion step, and the clone loses its skills.
- **Consequence:** contributors without Nix install the latest release, so their skill text can differ slightly from the Nix route. Git ignores the generated skills, so the difference cannot reach a commit. Each `SKILL.md` records its CLI version in `generatedBy`. ADR-013 accepts the same kind of drift between the two routes for Node.
- **An older CLI:** 1.12.0 generated the same 8 skills, with slightly different text. A CLI too old to know the `agents` tool stops with its own error, which lists the tools it knows.
- **Alternative — a pin in the `justfile`, checked on every run:** this was the first version. It had the two problems above.
- **Alternative — build the CLI at a chosen version in `nix/openspec.nix`:** one pin would then serve both routes. Each version bump would need a new source hash, which costs more than the drift.
- **Alternative — an npm devDependency:** it would add v2 tooling to the v0.10.x `package.json`, which the cutover replaces. It would also put a second `openspec` on `PATH` in the Nix shell.
- **Alternative — `npx @fission-ai/openspec@<version>`:** the first run needs the network, which conflicts with running offline on shell entry (#275).

### D6. Put the steps in `scripts/skills.sh`

The `skills` recipe is one line that calls `scripts/skills.sh`. The script uses `#!/usr/bin/env bash` with `set -euo pipefail`.

- **Why a script:** the steps take about 20 lines of shell. A recipe reads well inline up to about 5–10 lines. Beyond that, a separate file gets shell highlighting in editors and can be linted. The `check` recipe already calls `backend/scripts/check-arch.sh` in the same way.
- **Why `scripts/` at the repo root:** the script serves both stacks, so it does not belong under `backend/`.
- **Why one script with `set -e`:** the steps depend on each other. A failed check must stop the run before any file is deleted.

### D7. Keep every deletion inside the repo

Before it deletes anything, the script resolves the real paths of `.agents` and `.agents/skills`. It stops with an error unless both lie inside the repo root.

- **Why:** a symbolic link at `.agents` or `.agents/skills` could otherwise point the deletion at a directory outside the repo. A contributor might link `.agents/skills` to a personal skills folder, for example. Checking each level covers a link at either one.
- **Alternative — check only `.agents/skills`:** it misses a link one level up, at `.agents`.
- **Why not `.config`:** the script only reads that folder, and the CLI writes nothing there. A link in place of the tracked `.config` folder shows in `git status`.

## Risks / Trade-offs

- **A future CLI renders `claude` skills differently** → Claude would read the generic text, which still works. The spec scenario "Claude sees the generated and authored skills" and #276's drift check are where to catch it. Move Claude to its own folder if that happens.
- **Contributors without Nix get slightly different skill text** → accepted, as D5 explains. `generatedBy` shows which version made each skill.
- **A future CLI writes to its config file, or adds files beside it** → `git status` shows the tracked file as changed, or the new files as untracked. The scenario "The CLI does not write to the repo's settings" fails, and the fix is decided then. Ignoring the folder would hide these writes, so git does not ignore it.
- **A contributor replaces `.config` with a link to another folder** → the CLI reads the settings found there, so the skills can differ. `git status` shows the tracked config as missing or changed. The repo does not support this setup.
- **Someone runs `openspec init` or `openspec update` by hand** → it renders with their global settings into ignored files. The next `just skills` restores the repo's settings. `AGENTS.md` tells agents to use `just skills`.
- **An authored skill is named `openspec-something`** → `just skills` deletes it. The docs reserve the prefix.
- **A run fails or stops after the deletion step** → the clone has no generated skills until the next successful run, and git cannot restore them. When a step fails, the script says to run `just skills` again, or to update `openspec` if the CLI failed. A killed run prints nothing, but the same rerun fixes it. A new run needs no network and takes seconds. We accept this state rather than generate into a copy and swap it in. `init` needs a full project around it, so the swap would add more code than the problem costs.

## Migration Plan

1. Merge the change. When a contributor pulls it, git deletes the tracked generated skills and the marker from their working tree.
2. Each contributor runs `just skills` once. Without the CLI, the command tells them what to install.
3. Until they run it, agents in that clone see only the authored skills.

**Rollback:** revert the merge commit. The generated skills and the marker return to git.
