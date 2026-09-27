## 1. The `just skills` recipe

- [x] 1.1 Add `openspec_version := "1.13.1"` and the 8-workflow list to the `justfile`. Add a `skills` recipe, written as one bash script with `set -euo pipefail` (design D5 and D6), whose first step compares `openspec --version` with the pin. Verify with a stub `openspec` early on `PATH` that prints `1.12.0`:
  - `just skills` exits with a non-zero status;
  - the error names `1.12.0`, `1.13.1` and `npm install -g @fission-ai/openspec@1.13.1`;
  - `git status --porcelain` shows the same output as before the run.
- [x] 1.2 Add the path-boundary check (design D7): before any deletion or write, the recipe resolves each target's parent directory and stops unless it lies inside the repo root. Verify both cases in a throwaway clone:
  - with `.agents` replaced by a link to a folder under `.workspace/`, `just skills` exits non-zero and no file in that folder changes;
  - with `.config` as a link to a folder outside the repo, the result is the same.
- [x] 1.3 Make the recipe delete `.agents/skills/openspec-*` and `.agents/skills/.openspec-target` (design D3). It then removes `.config/openspec`, creates it fresh, and writes `config.json` from the `justfile` values (design D2). Verify:
  - with `.config/openspec` as a link to another folder, that folder is unchanged after the run, and `.config/openspec` is a real directory;
  - `config.json` holds `profile: custom`, `delivery: skills` and the 8 workflows.
- [x] 1.4 Make the recipe run `openspec init --tools agents --no-animation` with `XDG_CONFIG_HOME` and `OPENSPEC_TELEMETRY=0` as a prefix on that command only (design D1). Above it, add a comment that links Fission-AI/OpenSpec#914 and #779 and says to remove the redirect once the CLI reads project-scoped `profile`, `delivery` and `workflows`. Verify:
  - `.agents/skills` holds exactly the 8 expected `openspec-*` folders;
  - `grep -rl '\$openspec-' .agents/skills` finds nothing;
  - every generated `SKILL.md` has `generatedBy: "1.13.1"`;
  - `config.json` has no `telemetry` key after the run;
  - `echo $XDG_CONFIG_HOME $OPENSPEC_TELEMETRY` in the calling shell prints the same as before the run.
- [x] 1.5 Make the recipe print "run `just skills` again" when a step fails after the version check. Verify with a stub `openspec` that reports `1.13.1` but fails on `init`: the message appears and the exit status is non-zero.

## 2. Git holds only authored skills

- [x] 2.1 Add to `.gitignore`: `/.agents/skills/openspec-*/`, `/.agents/skills/.openspec-target` and `/.config/openspec/`. Verify that `git check-ignore` reports each of the three paths as ignored, and reports `.agents/skills/critique/SKILL.md` as not ignored.
- [x] 2.2 Remove the 8 generated skill folders and the marker from git with `git rm -r --cached`, then run `just skills`. Verify:
  - `git status --porcelain` shows only the intended removals, and nothing untracked;
  - `git ls-files .agents/skills` lists no `openspec-*` path and no `.openspec-target`.
- [x] 2.3 Verify that the authored skills survive generation. Record `sha256sum` of every file under `critique`, `grill-me`, `choose-an-adversary` and `discovery`, run `just skills` twice, and confirm that the checksums are unchanged. Also confirm that `diff -r` of `.agents/skills` finds no difference between the two runs.
- [x] 2.4 Verify that Claude sees the full set: `.claude/skills/openspec-propose/SKILL.md` and `.claude/skills/critique/SKILL.md` both exist, and `realpath .claude/skills` equals `realpath .agents/skills`.
- [x] 2.5 Verify that the run touches nothing else. Compare `git status --porcelain --ignored` before and after `just skills`: the only new entries are under `.agents/skills` and `.config/openspec`.

## 3. Same output on every machine

- [ ] 3.1 Verify that the global config has no effect. Run `just skills` in two throwaway clones of one commit. Give each run a different `HOME` whose `.config/openspec/config.json` differs: one uses the `core` profile, and one sets `delivery: both`. Confirm that `diff -r .agents/skills` between the clones finds no difference, and that neither `HOME` config file changed.
- [ ] 3.2 Verify the stale-marker case. In a throwaway clone, put back the old tracked `openspec-*` skills and a `.openspec-target` containing `codex`, then run `just skills`. Confirm that `grep -rl '\$openspec-' .agents/skills` finds nothing.
- [ ] 3.3 Verify that removing a workflow removes its skill. Temporarily drop `verify` from the `justfile` list and run `just skills`. Confirm that `.agents/skills/openspec-verify-change` no longer exists. Then revert the change and run `just skills` again.
- [ ] 3.4 Verify the route without Nix. Outside the Nix shell, install `@fission-ai/openspec@1.13.1` into a prefix under `.workspace/` and put it first on `PATH`. Run `just skills`, and confirm that `diff -r .agents/skills` against a `nix develop --command just skills` run finds no difference.

## 4. Documentation

- [ ] 4.1 Update `CONTRIBUTING.md` under "Development Environment". Cover:
  - `just skills` as a setup step on both routes;
  - installing the pinned CLI without Nix, with `npm install -g @fission-ai/openspec@<version>`;
  - the reserved `openspec-` prefix;
  - that the recipe owns `.config/openspec`;
  - how to add an agent: nothing, a folder link, or a separate generated set.

  Verify that `grep` finds `just skills`, `openspec-` and `@fission-ai/openspec@1.13.1` in the file.
- [ ] 4.2 Update `AGENTS.md`. Tell agents to run `just skills` instead of `openspec init` or `openspec update`, and state the reserved prefix. Verify that `grep` finds `just skills` in `AGENTS.md`.
- [ ] 4.3 Check the new prose against ISO 24495-1: no sentence over 30 words, and active voice with the actor named. Verify with a sentence-length scan of the changed lines.

## 5. Gate

- [ ] 5.1 Verify the whole repo: `nix develop --command just skills`, `just check`, `nix flake check` and `openspec validate generated-agent-skills --strict` each exit with status 0.
