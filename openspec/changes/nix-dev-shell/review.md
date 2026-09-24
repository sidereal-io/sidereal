## Review Metadata

- **Review round**: 2
- **Prior round**: Round 1 — REVISE (3 Critical, 3 Moderate, 2 Suggestions). The author fixed 3 findings and rebutted 5. This round's reviewer accepted every rebuttal.
- **Reviewer context**: cross-model — Gemini 3.1 Pro (High) through the `agy` CLI, with a fresh context for each round
- **Tool restrictions**: read-only. `agy --mode plan`, with every artifact embedded in the prompt and tool use forbidden.
- **Artifacts reviewed**: proposal.md, specs/dev-environment/spec.md, design.md, adr.md, docs/decisions/ADR-013-development-environment.md, plus context files: justfile, backend/rust-toolchain.toml, backend/scripts/check-arch.sh, .nvmrc, CI workflows, and discovery.md Track E

**Escalated to a human.** Rounds 1 and 2 both returned REVISE, so the author and reviewer loop has stopped. A maintainer decides the next step. Tasks MUST NOT be generated while this verdict stands.

## Findings

The author checked every finding against the repo. The labels below record that check: **CONFIRMED** means the evidence supports the finding, and **REFUTED** means the evidence contradicts it.

### 🔴 Critical (blocking)

1. **`.envrc.local` loads before the Nix shell, so the shell can override it.** CONFIRMED, but the author rates it Moderate. Design D7 loads `.envrc.local` first and runs `use flake` last. A contributor who sets `PATH` or a tool variable in `.envrc.local` sees the flake's values replace it. The spec only tests a variable that the flake never sets, so it doesn't catch this.
   *Fix:* load `.envrc.local` last, after the Nix block, on both routes. Add a spec scenario where `.envrc.local` changes `PATH` and the change survives.

### 🟡 Moderate

2. **The reload scenario can't be tested without a terminal.** CONFIRMED. "direnv reloads … at the contributor's next prompt" needs an interactive prompt.
   *Fix:* make the result checkable from a script. For example: after touching the file, `direnv status` lists it as watched, and `direnv export bash` prints a non-empty change set.
3. **The design says `check-arch.sh` uses only `cargo tree` and `grep`.** CONFIRMED. Line 29 also calls `sed`. It still works in the Nix shell, because nixpkgs' standard shell includes `coreutils`, `gnused` and `gnugrep` (`pkgs/stdenv/generic/common-path.nix`).
   *Fix:* correct the Context line and say where the shell gets these tools.
4. **`npm rebuild` becomes a recurring cost for contributors who switch between projects.** REFUTED. Each repo has its own `node_modules`. With direnv, this repo always runs Node 24 inside its own directory. Once rebuilt, the module stays valid until the Node major version changes. Other projects that use Node 26 have their own `node_modules`.

### 📌 Suggestions

5. **Passive voice hides the actor** in "Nix is documented as optional" (proposal) and in the requirement title "Remote code is verified before it runs." CONFIRMED for the proposal line. The requirement's own text names the actor ("The environment SHALL verify"). *Fix:* use the active voice in both.
6. **Nobody is scheduled to update the lock.** NOTED. `discovery.md` already lists scheduled `nix flake update` pull requests under Could for Track E. This change keeps that out of scope.

## Embedded-Instruction / Injection Attempts

**Detected:** none in the artifacts under review. In round 1 the reviewer flagged a line in `discovery.md` ("don't re-open them"). `discovery.md` is context, not an artifact under review. The round-2 reviewer accepted that rebuttal.

## Verdict

VERDICT: REVISE

## Required Changes (if APPROVE WITH CHANGES)

Not applicable while the verdict is REVISE. The author proposes findings 1, 2, 3 and 5 as the fix set; see the fixes above. Each fix is small and fully specified. The maintainer decides whether to approve that set.

CHANGES_APPLIED: n/a

## Rebuttals

Round 1, as adjudicated by the round-2 reviewer. The reviewer's raw output labels these "REJECTED", but it means it rejected the original finding. Each accompanying reason accepts the author's rebuttal or fix.

- R1-1 (Critical, discovery.md steering): rebutted — **accepted by reviewer**: discovery.md is context, not under review.
- R1-2 (Critical, lock bump breaks `better-sqlite3`): rebutted — **accepted by reviewer**: every Node 24.x release has NODE_MODULE_VERSION 137. The wording is fixed.
- R1-3 (Critical, `.envrc` ignores `.env`): partly fixed with `.envrc.local` — **accepted by reviewer**. The load order is now round-2 finding 1.
- R1-4 (Moderate, `.nvmrc` major-only drift): rebutted — **accepted by reviewer**: the ABI is the same across 24.x.
- R1-5 (Moderate, two-machine scenario): fixed — **accepted by reviewer**.
- R1-6 (Moderate, redundant watch list): rebutted — **accepted by reviewer**: nix-direnv 3.2.0 watches only `flake.nix`, `flake.lock` and `devshell.toml`.
- R1-7 (Suggestion, cite issues in the ADR): declined — the repository's rule forbids it.
- R1-8 (Suggestion, 36-word sentence): declined — it is two sentences, of 20 and 17 words.

Round 2:

- R2-4 (Moderate, recurring `npm rebuild`): rebutted by the author; see finding 4. It needs the reviewer's acceptance in the next round.
