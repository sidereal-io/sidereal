## 1. The `just skills` script

- [ ] 1.1 Create `scripts/skills.sh`, an executable bash script with `set -euo pipefail` (design D6). Its first step stops unless `openspec` is on `PATH` (design D5). Replace the `justfile` recipe with the one line `scripts/skills.sh`, and remove `openspec_version` and `openspec_workflows`. Verify with a `PATH` that has no `openspec`:
  - `just skills` exits with a non-zero status;
  - the error names `npm install -g @fission-ai/openspec`;
  - `git status --porcelain` shows the same output as before the run.
- [ ] 1.2 Add the path-boundary check for `.agents` and `.agents/skills` (design D7). Verify in throwaway clones with a stub `openspec`:
  - with `.agents` as a link to a folder outside the clone, `just skills` exits non-zero and no file in that folder changes;
  - with `.agents/skills` as a link to a folder outside the clone, the result is the same.
- [ ] 1.3 Make the script delete `.agents/skills/openspec-*` and `.agents/skills/.openspec-target` (design D3). It then runs `openspec init --tools agents --no-animation`, with `XDG_CONFIG_HOME="$root/.config"` and `OPENSPEC_TELEMETRY=0` as a prefix on that command only (design D1). Above it, keep the comment that links Fission-AI/OpenSpec#914 and #779. Verify:
  - `.agents/skills` holds exactly the 8 expected `openspec-*` folders;
  - `grep -rl '\$openspec-' .agents/skills` finds nothing;
  - every generated `SKILL.md` has `generatedBy` equal to the output of `openspec --version`;
  - `git diff --exit-code .config/openspec/config.json` exits 0;
  - `echo $XDG_CONFIG_HOME $OPENSPEC_TELEMETRY` in the calling shell prints the same as before the run;
  - `shellcheck scripts/skills.sh` reports nothing.
- [ ] 1.4 Make the script print a message when a step fails after the CLI check. The message says to run `just skills` again, or to update `openspec` if the CLI failed. Verify with a stub `openspec` that fails on `init`: the message appears and the exit status is non-zero.

## 2. Git tracks authored skills and the settings

- [ ] 2.1 Track `.config/openspec/config.json` with `profile: custom`, `delivery: skills` and the 8 workflows (design D2). Remove `/.config/openspec/` from `.gitignore`. Verify:
  - `git ls-files .config/openspec` prints only `.config/openspec/config.json`;
  - `git check-ignore` still reports `.agents/skills/openspec-propose/SKILL.md` and `.agents/skills/.openspec-target` as ignored.
  - `git ls-files .agents/skills` lists no `openspec-*` path and no `.openspec-target`.
- [ ] 2.2 Verify that generating leaves git clean. In a fresh clone, run `just skills`, and confirm that `git status --porcelain` prints nothing.
- [ ] 2.3 Verify that the authored skills survive generation. Record `sha256sum` of every file under `critique`, `grill-me`, `choose-an-adversary` and `discovery`, run `just skills` twice, and confirm that the checksums are unchanged. Also confirm that `diff -r` of `.agents/skills` finds no difference between the two runs.
- [ ] 2.4 Verify that Claude sees the full set: `.claude/skills/openspec-propose/SKILL.md` and `.claude/skills/critique/SKILL.md` both exist, and `realpath .claude/skills` equals `realpath .agents/skills`.

## 3. Same output on every machine

- [ ] 3.1 Verify that the global config has no effect. Run `just skills` in two throwaway clones of one commit, with the same CLI. Give each run a different `HOME` whose `.config/openspec/config.json` differs: one uses the `core` profile, and one sets `delivery: both`. Confirm that `diff -r .agents/skills` between the clones finds no difference, and that neither `HOME` config file changed.
- [ ] 3.2 Verify the stale-marker case. In a throwaway clone, put back the old tracked `openspec-*` skills and a `.openspec-target` containing `codex`, then run `just skills`. Confirm that `grep -rl '\$openspec-' .agents/skills` finds nothing, and that the skills match a clean clone's.
- [ ] 3.3 Verify that removing a workflow removes its skill. In a throwaway clone, drop `verify` from `.config/openspec/config.json` and run `just skills`. Confirm that `.agents/skills/openspec-verify-change` no longer exists.
- [ ] 3.4 Verify the route without Nix:
  - with `@fission-ai/openspec@1.13.1` installed into a prefix under `.workspace/` and first on `PATH`, `just skills` exits 0, and `diff -r .agents/skills` against a `nix develop --command just skills` run finds no difference;
  - with an older CLI, 1.12.0, `just skills` exits 0, generates the 8 folders, and no file contains `$openspec-`.

## 4. Documentation

- [ ] 4.1 Update `CONTRIBUTING.md`:
  - the setup without Nix installs `@fission-ai/openspec` with no version, and says the Nix shell provides the tested version;
  - the "Agent skills" section drops the version check and the rule that the recipe owns `.config/openspec`;
  - it says the workflow list lives in `.config/openspec/config.json`, and that `openspec/config.yaml` holds the schema, context and rules.

  Verify that `grep` finds `just skills`, `openspec-` and `.config/openspec/config.json` in the file, and finds no `1.13.1`.
- [ ] 4.2 Check `AGENTS.md` against the revised plan, and update any statement it contradicts. Verify that `grep` finds `just skills` in `AGENTS.md`.
- [ ] 4.3 Check the new prose against ISO 24495-1: no sentence over 30 words, and active voice with the actor named. Verify with a sentence-length scan of the changed lines.

## 5. Gate

- [ ] 5.1 Verify the whole repo: `nix develop --command just skills`, `just check`, `nix flake check` and `openspec validate generated-agent-skills --strict` each exit with status 0.
