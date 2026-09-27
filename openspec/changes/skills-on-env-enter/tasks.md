## 1. The stamp and the stale check

- [x] 1.1 Make `scripts/skills.sh` delete `.agents/skills/.openspec-stamp` together with the other generated files, before `init` (design D2). After `init` succeeds, make it write the stamp to a temporary file and rename it into place. The stamp holds one `cli` line, one `config` line, one `script` line and one `skill` line per generated folder, as design D2 describes. Verify after `just skills`:
  - the `cli` line equals `cli ` plus the output of `openspec --version`;
  - the `config` and `script` lines match `git hash-object` of each file;
  - the `skill` lines name exactly the 8 generated folders;
  - `shellcheck scripts/skills.sh` reports nothing.
- [x] 1.2 Add `/.agents/skills/.openspec-stamp` to `.gitignore`, beside the existing generated-skill rules. Verify that `git check-ignore .agents/skills/.openspec-stamp` reports it, and that `git status --porcelain` prints nothing after `just skills`.
- [x] 1.3 Add the `--if-stale` option to `scripts/skills.sh` (design D2). With it, the script exits 0 without changing any file when the stamp matches the current inputs and every listed folder holds a `SKILL.md`. Otherwise it runs in full. Verify in a throwaway clone, starting each case from a fresh `just skills`:
  - nothing changed: `scripts/skills.sh --if-stale` leaves every modification time under `.agents/skills` unchanged;
  - no stamp: the run regenerates and writes a stamp;
  - `verify` removed from `.config/openspec/config.json`: the run removes `openspec-verify-change`;
  - a comment added to `scripts/skills.sh`: the run regenerates;
  - `.agents/skills/openspec-explore` deleted: the run recreates it;
  - a stub `openspec` first on `PATH` that prints a different version for `--version` and passes other calls to the real CLI: the run regenerates.
- [x] 1.4 Verify that a failed full run leaves no stamp. With a stub `openspec` that exits 1 on `init`, run `just skills`, and confirm that `.agents/skills/.openspec-stamp` does not exist.

## 2. The `enter` recipe

- [x] 2.1 Add the `enter` recipe to the `justfile`, as design D3 shows, with a comment that says the shell runs it on load. Verify:
  - with stale skills, `just enter` exits 0 and prints nothing to standard output or standard error;
  - with a stub `openspec` that exits 1, `just enter` exits 0, and standard error holds exactly one line that names `just skills`;
  - `just --list` shows the recipe with its comment.

## 3. The Nix shell

- [ ] 3.1 Add the generic hook to `nix/devshell.nix` (design D1). Order it after the other hooks with `lib.mkAfter`. It runs `just enter` only when `just` is on `PATH` and `just --show enter` succeeds, and it ends with `|| true`. Update the `hooks` option description, which still says a later story adds the first hook. Verify:
  - with stale skills, `nix develop --command true` exits 0, prints nothing from the hook, and leaves the 8 generated folders;
  - with the `enter` recipe removed in a throwaway clone, `nix develop --command true` exits 0 and the hook prints nothing;
  - `nix/devshell.nix` contains no mention of `skills` or `openspec`.
- [ ] 3.2 Make `nix/openspec.nix` add a hook that exports `OPENSPEC_NO_UPDATE_CHECK=1` (design D4). Verify that `nix develop --command printenv OPENSPEC_NO_UPDATE_CHECK` prints `1`, and that `direnv exec . printenv OPENSPEC_NO_UPDATE_CHECK` prints `1`.
- [ ] 3.3 Verify the refresh works offline. On Linux, delete the stamp, then run `unshare --net --map-root-user nix develop --offline --command true`. Confirm that `.agents/skills` holds the 8 generated folders and a new stamp.

## 4. direnv

- [ ] 4.1 Add `watch_file .config/openspec/config.json scripts/skills.sh` to `.envrc`, outside the `if has nix` block (design D5), with a one-line comment on why. Verify that `direnv status` in the repo lists both files as watched.
- [ ] 4.2 Verify the direnv scenarios in a throwaway clone with Nix and direnv, after `direnv allow`:
  - fresh clone: `direnv exec . true` leaves the 8 generated folders, and `git status --porcelain` prints nothing;
  - nothing changed: a second `direnv exec . true` leaves every modification time under `.agents/skills` unchanged;
  - folder deleted: after deleting `.agents/skills/openspec-explore`, `direnv exec . true` recreates it;
  - settings changed: after removing `verify` from `.config/openspec/config.json`, `direnv exec . true` removes `openspec-verify-change`.
- [ ] 4.3 Verify the opt-in without Nix. In a throwaway clone, put a `PATH` without Nix but with `just` and an npm-installed `openspec`, write `just enter` in `.envrc.local`, and delete `.agents/skills/openspec-explore`. Run `direnv exec . true`, and confirm that `openspec-explore/SKILL.md` exists again.

## 5. Documentation

- [ ] 5.1 Update `CONTRIBUTING.md`:
  - the Nix setup says the shell refreshes the skills when it loads, and only when they are stale; drop "run `just skills` once after you clone" from that route;
  - the setup without Nix explains the `just enter` line for `.envrc.local`;
  - the "Agent skills" section says `just skills` still forces a full run;
  - a short note tells contributors to run `direnv deny` before they check out a branch they don't trust, because loading the shell runs code from the working tree.

  Verify that `grep` finds `just enter`, `direnv deny` and `.envrc.local` in the file.
- [ ] 5.2 Check `AGENTS.md` against the new behavior, and update any statement it contradicts. Verify that `grep -n 'just skills' AGENTS.md` still finds the rule to generate skills with `just skills`.
- [ ] 5.3 Check the new prose against ISO 24495-1: no sentence over 30 words, and active voice with the actor named. Verify with a sentence-length scan of the changed lines.

## 6. Gate

- [ ] 6.1 Verify the whole repo: `just check`, `nix flake check`, `shellcheck scripts/skills.sh` and `openspec validate skills-on-env-enter --strict` each exit with status 0, and `git status --porcelain` prints nothing after `nix develop --command true`.
