Origin: [#275](https://github.com/sidereal-io/sidereal/issues/275) (`skills-on-env-enter`).

## Why

The generated OpenSpec skills go stale, and nothing tells anyone to refresh them:

- **A fresh clone has no generated skills.** Git ignores them, so they exist only after someone runs `just skills`.
- **A pull can change their inputs.** A new CLI pin in `flake.lock`, a new workflow list, or a new `scripts/skills.sh` leaves the old skills in place.
- **Agents don't know the rule.** An agent reads whatever sits in `.agents/skills`. It has no way to know the files are out of date.

The skills behave like `node_modules`: a pinned tool makes the same output every time, but only when someone runs it. This change runs it for you when you enter the shell, and only when its inputs changed.

## What Changes

- **The shell runs one generic hook.** When the Nix shell loads, it runs `just enter` if the repo's `justfile` has an `enter` recipe. The hook never stops the shell from loading, and it prints nothing on success.
- **A new `enter` recipe.** It refreshes the skills only when they are stale. A failed refresh prints one warning line that says to run `just skills`, and `enter` still succeeds.
- **`just skills` writes a stamp.** After a successful run, it records what the skills were made from:
  - the CLI version;
  - the contents of `.config/openspec/config.json` and `scripts/skills.sh`;
  - the list of generated skill folders.

  A stale check compares the stamp with the current inputs. It also treats a missing stamp or a missing skill folder as stale.
- **The shell turns off the CLI's update check.** The shell exports `OPENSPEC_NO_UPDATE_CHECK=1`, so `openspec` commands run in the shell never contact the npm registry.
- **direnv watches the skill inputs.** `.envrc` also watches `.config/openspec/config.json` and `scripts/skills.sh`. A pull that changes either one reloads the shell, which refreshes the skills.
- **Docs explain the refresh.** `CONTRIBUTING.md` says when the refresh runs. It also tells contributors without Nix how to get the same refresh through `.envrc.local`.

**Scope change from #275.** The issue puts the hook in the OpenSpec flake module, `nix/openspec.nix`. This change puts it in `nix/devshell.nix` instead, and makes it call `just enter`. The Nix modules are written so a shared flake can take them over unchanged. A hook that names this repo's `skills` recipe would break that. `design.md` gives the reasons.

Nothing here is **BREAKING**. Contributors who run `just skills` by hand can keep doing so.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `dev-environment`: adds requirements for refreshing the skills when the shell loads. They cover the stale check, a hook that never blocks the shell, an offline CLI, and the reload after a pull. The existing requirement "The shell turns on when you enter the repo" gains the new watched files.

## Impact

- **Changed files:**
  - `nix/devshell.nix` — adds the generic `just enter` hook;
  - `nix/openspec.nix` — exports `OPENSPEC_NO_UPDATE_CHECK=1`;
  - `justfile` — adds the `enter` recipe;
  - `scripts/skills.sh` — writes the stamp, and gains a stale-only mode;
  - `.envrc` — watches the two skill input files;
  - `.gitignore` — ignores the stamp;
  - `CONTRIBUTING.md` and `AGENTS.md`.
- **Not affected:**
  - what `just skills` generates;
  - the authored skills and the `.claude/skills` link;
  - contributors with direnv but without Nix, unless they opt in through `.envrc.local`;
  - application code in either stack;
  - CI. The drift check is a later story (#276).
