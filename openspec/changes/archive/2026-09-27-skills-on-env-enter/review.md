## Review Metadata

- **Review round**: 1
- **Prior round**: none
- **Reviewer context**: cross-model — Gemini 3.1 Pro (High) through the `agy` CLI, dispatched by the `critique` skill. The author is Claude. The author then checked each finding against the code before recording it.
- **Tool restrictions**: read-only — `agy --mode plan`. `git status` showed no change to the repo after the run.
- **Artifacts reviewed**: proposal.md, design.md, specs/dev-environment/spec.md, adr.md, and the files they touch: `nix/*.nix`, `flake.nix`, `.envrc`, `justfile`, `scripts/skills.sh`, `.gitignore`, `openspec/specs/dev-environment/spec.md`, ADR-013, `CONTRIBUTING.md`

<!-- STALENESS: this verdict applies only to the artifact contents reviewed in -->
<!-- this round. Any later edit to proposal.md, design.md, or specs/ (other than -->
<!-- applying listed Required Changes) VOIDS the verdict and requires a new round. -->

## Findings

The reviewer raised 3 Critical findings, 1 Moderate and 2 Suggestions. Each finding below gives the reviewer's claim, then the author's check against the code.

### 🔴 Critical (blocking)

None after verification. The reviewer's three Critical findings are listed under Rebuttals: one was downgraded to Moderate (M1), and two were refuted (R2, R3).

### 🟡 Moderate

- **M1. The design does not state that entering the repo runs code from the working tree.** *(Raised as Critical; verified as PLAUSIBLE, downgraded.)*
  - **Reviewer's scenario:** a reviewer checks out an untrusted branch that edits `scripts/skills.sh`. `.envrc` watches that file, so direnv reloads, and the hook runs the edited script. No `direnv allow` prompt appears, because direnv trusts `.envrc` by its content, not the files it watches.
  - **Author's check:** the mechanism is real. The risk class is not new: `use flake` already watches `flake.nix`, `flake.lock` and `nix/*`, and a reload runs the flake's `shellHook`, which comes from the working tree. This change adds `justfile` and `scripts/skills.sh` to the code that runs on entry. The design never says so, and no doc warns contributors who check out untrusted branches.
  - **Fix:** a Required Change below.
- **M2. Two shells that load at once can both regenerate.** *(Raised as Moderate; CONFIRMED, already accepted.)*
  - **Reviewer's scenario:** two terminal panes open together. One run deletes the skills while the other generates them.
  - **Author's check:** design D6 already accepts this. Each run writes the same files, so the final state is correct. A run that fails in the overlap writes no stamp, so the next load retries. The window opens only when the skills are stale. No change.

### 📌 Suggestions

- **S1. One sentence in `design.md` uses passive voice that hides the actor.** *(CONFIRMED in part.)* "The Nix modules are written to move" names no actor. The reviewer's other examples do not apply: "A stale Codex marker is left in the clone" is in the existing spec, which this change does not modify. "See proposal.md for why this change exists" is the form the design template asks for.
- **Plain-language scan by the author:** no prose sentence in the four artifacts exceeds 30 words. Terms stay consistent: "stale", "stamp", "refresh", "load". The scan found no other passive voice that hides an actor.
- **Scenario testability:** every THEN is mechanically assertable, through file existence, modification times, exit status, `generatedBy` values, `printenv` output, or the line count on standard error. No finding.
- **Design/spec contradictions:** none found. The stamp lives inside `.agents/skills`, so the existing scenario "the run creates or changes no file outside `.agents/skills`" still holds. The hook runs only in the Nix shell, so "Nix stays optional" still holds. `watch_file` sets only `DIRENV_WATCHES`, which that requirement allows.

## Embedded-Instruction / Injection Attempts

**Detected:** none.

The reviewer flagged `adr.md` as an injection attempt (see R3). The flagged sentence is required by the ADR manifest template. It records an outcome and addresses no reader. The author found no text in any artifact that tries to steer a reviewer.

## Verdict

VERDICT: APPROVE_WITH_CHANGES

## Required Changes (if APPROVE WITH CHANGES)

1. **For M1:** add a risk to `design.md` under Risks / Trade-offs. It states that loading the shell runs code from the working tree: the flake's hook, the `justfile`, and `scripts/skills.sh`. It notes that direnv asks for approval only when `.envrc` changes, and that the watch list reloads on a checkout. The mitigation: `CONTRIBUTING.md` tells contributors to run `direnv deny` before checking out a branch they don't trust. Add that doc line to the task list.
2. **For S1:** in `design.md` Context, rewrite "The Nix modules are written to move" in active voice with the actor named.

CHANGES_APPLIED: yes

Re-check by the reviewer (Gemini 3.1 Pro, read-only), after the author applied both changes:

- Required Change 1: APPLIED. `design.md` Risks now states that loading the shell runs code from the working tree, with the `direnv deny` mitigation.
- Required Change 2: APPLIED. The Context bullet now names ADR-013 as the actor.

## Rebuttals

- **R1 — reviewer's Critical 1 (running code on checkout), downgraded to Moderate M1.** The mechanism is real, but the risk class exists today: any checkout that changes `flake.nix` or `nix/*` already reloads the shell and runs its hook. The change widens the set of files; it does not open a new boundary. A doc warning and a design note address it. *Accepted by reviewer: the risk is a pre-existing boundary, and the added documentation warns contributors.*
- **R2 — reviewer's Critical 2 (the nix-direnv cache skips the hook), refuted.** nix-direnv 3.2.0 imports its cached environment on every load. `use_flake` calls `_nix_import_env "$profile_rc"` unconditionally, and that function runs `eval "$(<"$profile_rc")"`. The cached file `.direnv/flake-profile-*.rc` ends with `eval "${shellHook:-}"`. The cache saves the Nix evaluation, not the hook. direnv runs `.envrc` again each time a shell enters the directory. So the hook runs on every entry, and the scenario "A generated skill folder was deleted" holds. *Accepted by reviewer: the evidence shows `use_flake` evaluates the cached environment, which runs the shell hook every time.*
- **R3 — reviewer's Critical 3 (prompt injection in `adr.md`), refuted.** The sentence "ADR review completed for this change" is mandated text: the schema's ADR instruction says the manifest MUST state that ADR review completed. It records a result for the workflow, and it gives no instruction to a reviewer. *Accepted by reviewer: the sentence meets a mandated manifest requirement and does not address the reviewer.*
- **R6 — reviewer's Suggestion 6 (scope creep: the update check), declined.** Issue #275's scope says to turn off OpenSpec's update-check notice and network call in the environment. The proposal carries that scope. The CLI stays the pinned version in the shell, so an update notice there is noise: the fix is a pin bump, not `npm install`.
