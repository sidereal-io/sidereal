## Context

See `proposal.md` for why this change exists. The requirements are in `specs/dev-environment/spec.md`. This design records how the refresh meets them, and why.

The chain from entering the repo to an agent reading a skill:

```
  cd sidereal
    --> direnv reads .envrc ("use flake")
    --> nix-direnv loads the cached Nix shell
    --> the shell's hooks run          (nix/devshell.nix: `hooks`, empty today)
    --> just enter                     (justfile)          <-- this change
    --> scripts/skills.sh --if-stale                       <-- this change
    --> openspec init --tools agents   (only when stale)
    --> .agents/skills/openspec-*      (ignored by git)
    <-- every agent in the directory reads these files
```

Facts that shape the approach. Each was checked against OpenSpec 1.13.1, the version the Nix shell provides:

- **The CLI has an off switch for its update check.** It skips the check when `OPENSPEC_NO_UPDATE_CHECK` is set, when `OPENSPEC_TELEMETRY=0`, or under CI. `scripts/skills.sh` already sets `OPENSPEC_TELEMETRY=0`, so today's generation makes no network call.
- **Generation is cheap.** `openspec init` takes about 0.3 seconds. Asking `openspec --version` costs about the same, because each starts Node.
- **`openspec update` is not an option.** It detects staleness on its own, but it has no `--tools` filter. In a test, it rewrote and deleted skills in a global `~/.minimax/skills` folder outside the repo. `AGENTS.md` already forbids it.
- **The Nix modules are written to move.** `nix/openspec.nix` says nothing in it is specific to Sidereal, so a later shared flake can take it over unchanged.
- **A shell hook reaches only shells that load the flake.** Agents started some other way don't run it. They still read the same files on disk, so one refresh serves every agent in the directory.

## Goals / Non-Goals

**Goals:**

- Loading the shell leaves current skills in `.agents/skills`, without anyone running a command.
- The Nix layer stays free of Sidereal-specific knowledge.
- The staleness logic lives beside the generation logic, in one file.

**Non-Goals:**

- A CI check for skill drift (#276).
- Refreshing skills for agents that never load the shell. They read the files the last refresh wrote.
- Checking that agents run the same `openspec` version that generated the skills.
- Detecting hand edits to a generated `SKILL.md`. `just skills` restores them, and #276 catches them in CI.
- Turning off OpenSpec telemetry for the whole shell. That is each contributor's choice.

## Decisions

### D1. The shell runs a generic `just enter` hook

`nix/devshell.nix` adds one hook, ordered after all other hooks. It runs `just enter` when `just` is on `PATH` and the `justfile` defines an `enter` recipe. Otherwise it does nothing. It ends with `|| true`, so no failure can stop the shell from loading.

- **Why generic:** the hook knows only that a repo may define an `enter` recipe. The `justfile` decides what entering means. The Nix modules stay portable, and later entry tasks need no Nix change.
- **Why check for the recipe:** a shared flake may serve a repo with no `enter` recipe. `just --show enter` answers in about 2 milliseconds.
- **Why last:** other hooks, such as the export in D4, then take effect before `enter` runs.
- **Alternative — a hook in `nix/openspec.nix` that runs `just skills`:** this is what #275 describes. The OpenSpec module would then assume this repo's recipe name, so a shared flake could not take it over unchanged.
- **Alternative — the stale check in Nix:** Nix would need to know the repo's skill inputs. The check would then be hard to test, and missing without Nix.

### D2. `scripts/skills.sh` writes a stamp and gains `--if-stale`

After a successful run, `scripts/skills.sh` writes `.agents/skills/.openspec-stamp`. The stamp is plain text with one entry per line:

- `cli`, followed by the output of `openspec --version`;
- `config`, followed by the `git hash-object` of `.config/openspec/config.json`;
- `script`, followed by the `git hash-object` of `scripts/skills.sh`;
- `skill`, followed by the folder name, once for each generated `openspec-*` folder.

The script writes the stamp to a temporary file, then renames it into place. A full run deletes the stamp first, together with the other generated files. A failed run therefore leaves no stamp, and the next load retries.

With `--if-stale`, the script builds the first three entries from the current inputs. It exits 0 without changing anything when they match the stamp and every listed folder holds a `SKILL.md`. Otherwise it runs in full.

- **Why a stamp, when checking costs as much as generating:** the stamp does not save time. It avoids rewriting skill files that a running agent may be reading, and it avoids churn on every shell load.
- **Why `git hash-object`:** Git is present wherever the repo is. `sha256sum` and `shasum` differ between Linux and macOS.
- **Why hash the script:** a change to how skills are generated must regenerate them.
- **Why not `openspec/config.yaml`:** the CLI reads no skill settings from it.
- **Why list the folders:** the check can find a deleted folder without mapping workflow names to folder names.
- **Why inside `.agents/skills`:** the stamp sits next to the files it describes. Deleting the folder removes the stamp too. The spec's promise that `just skills` writes only inside `.agents/skills` still holds.
- **Alternative — always regenerate:** simpler, but it rewrites every generated file on every load. An agent could briefly find its skills missing.
- **Alternative — `openspec update`:** it changes folders outside the repo (see Context).
- **Alternative — use the binary's resolved path as the CLI identity:** a Nix store path includes the version, but an npm install path does not.

### D3. The `enter` recipe hides detail and warns in one line

```
enter:
    @scripts/skills.sh --if-stale >/dev/null 2>&1 || echo "warning: could not refresh the OpenSpec skills; run \`just skills\` to see why" >&2
```

- **Why discard the script's output:** a refresh prints nothing when it works. When it fails, `just skills` shows the full error, and the warning names that command.
- **Why `enter` still succeeds after a failure:** the hook already ignores failures, but a contributor without Nix calls `enter` from `.envrc.local`. A failure there must not break their direnv load either.
- **Alternative — a `--quiet` flag in the script:** the redirect gives the same result with no new option.

### D4. `nix/openspec.nix` exports `OPENSPEC_NO_UPDATE_CHECK=1`

The OpenSpec module adds a hook that exports `OPENSPEC_NO_UPDATE_CHECK=1`. direnv captures variables that the shell's hooks export, so the variable reaches the contributor's shell.

- **Why:** it keeps every `openspec` command in the shell offline, including commands that agents run from the skills. Generation is already offline through `OPENSPEC_TELEMETRY=0`, so the export is not needed for the refresh itself.
- **Why in the OpenSpec module:** the setting is about the CLI, and nothing in it is specific to Sidereal.
- **Alternative — an `env` option in `nix/devshell.nix`:** it adds a new option for a single variable. A hook export does the same with the existing option.

### D5. `.envrc` watches the skill inputs

`.envrc` adds `watch_file .config/openspec/config.json scripts/skills.sh`, outside its Nix-only block.

- **Why:** direnv reloads only when a watched file changes. nix-direnv already watches `flake.nix` and `flake.lock`, so a CLI pin bump reloads the shell. The two skill inputs need their own watch, so a pull that changes them refreshes the skills without leaving the directory.
- **Why outside the Nix block:** a contributor without Nix who opts in through `.envrc.local` gets the same reload. `watch_file` changes only direnv's own `DIRENV_WATCHES` variable, which the "Nix stays optional" requirement allows.

### D6. Accept the race between two loading shells

Two shells that load at the same moment can both find the skills stale, and both regenerate. Each run deletes the generated files and writes the same ones again, so the result is correct. For a moment, an agent may find a skill missing. This happens only when the skills are stale.

- **Alternative — `flock`:** macOS does not ship it.
- **Alternative — a lock directory made with `mkdir`:** a killed run leaves the lock behind, which would block every later refresh.

## Risks / Trade-offs

- **Every shell load costs about 0.3 seconds more** → the stale check runs `openspec --version`. This includes `nix develop --command ...` calls. Accepted: it is the cost of a correct CLI identity (D2).
- **A hand-edited generated skill survives a load** → the stamp checks versions and folders, not file contents. `just skills` restores the file, and #276 catches the drift in CI.
- **A contributor without Nix opts in but has no `openspec`** → every load prints the one-line warning. `just skills` then names the install command.
- **The hook also runs in CI jobs that use `nix develop`** → it generates the skills there too. CI sets `CI`, so the CLI makes no network call. #276 can rely on that, or run `just skills` itself.
- **An agent reads a skill during a refresh** → the skill may be missing for about 0.3 seconds. This happens only when the inputs changed.

## Migration Plan

1. Merge the change.
2. On each contributor's next shell load, the missing stamp counts as stale. The shell regenerates the skills once and writes the stamp.
3. Later loads skip the refresh until an input changes.

**Rollback:** revert the merge commit. The shell stops refreshing, and `just skills` works as before. A leftover `.openspec-stamp` is ignored by git and harmless.
