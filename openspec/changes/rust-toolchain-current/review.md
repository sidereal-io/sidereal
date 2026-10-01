## Review Metadata

- **Review round**: 2
- **Prior round**: Round 1 (full review) ended APPROVE_WITH_CHANGES, with three Required Changes applied and re-checked. The reviewer had recommended REVISE on F1, F2 and F3; the author refuted F1 and fixed F2 and F3.
- **Why round 2**: after round 1, the maintainer decided that the change needs no new ADR. The author deleted the drafted ADR-014, corrected one consequence in ADR-013 in place, and edited design.md (D6), proposal.md (Impact) and adr.md. Those edits voided round 1's verdict. Round 2 reviewed only those edits, and was not a from-scratch review.
- **Reviewer context**: cross-model. Gemini (`gemini-3.1-pro-high`) through the `agy` CLI in plan mode, in a fresh session for each review and each re-check. The author is Claude.
- **Tool restrictions**: no tools. The first attempt, which let the reviewer read files, produced no output: headless plan mode denied a shell-command call. The author then put every reviewed file, and excerpts of the context files, into the prompt with their line numbers. The author also gave the reviewer the facts it could not check itself: the Nix, clippy and Docker test results from 2026-09-30, and what GitHub documents about Dependabot.
- **Artifacts reviewed**: round 1 covered proposal.md, design.md, specs/dev-environment/spec.md, adr.md, the drafted ADR-014, and a note added to ADR-013. Round 2 covered the diff of the no-ADR edits, plus ADR-013 and adr.md in full. Context: `nix/toolchains.nix`, `flake.nix`, the `rust-overlay` entry in `flake.lock`, `server/rust-toolchain.toml`, `server/Cargo.toml`, `server/Dockerfile`, `server/README.md`, `AGENTS.md`, `CONTRIBUTING.md`, `.github/dependabot.yml`, the `nix`, `v2` and `docker-build-push` workflows, the `dev-environment` spec, the personas in `openspec/discovery.md`, ADR-000 and ADR-001.

<!-- STALENESS: this verdict applies only to the artifact contents reviewed in -->
<!-- this round. Any later edit to proposal.md, design.md, or specs/ (other than -->
<!-- applying listed Required Changes) VOIDS the verdict and requires a new round. -->

## Findings

### Round 2

- **N1 (reviewer: Critical). ADR-013 would contradict itself.** The reviewer said the rewritten consequence clashes with ADR-013's Consequence 1: "Tools change only when someone updates that file in a pull request." Refuted. Consequence 1 describes the Nix environment. It was already untrue for contributors without Nix, because `.nvmrc` names only `26`, and ADR-013's original bullet said so. The edit puts Rust where Node already was. *Accepted by reviewer: "the edit expands an existing pattern rather than introducing a novel contradiction to Consequence 1."*
- **N2 (reviewer: Moderate). adr.md overstated ADR-013's scope.** It said the lock file pins every tool, but ADR-013 also keeps plain pin files for contributors without Nix. Confirmed and fixed: adr.md now names both routes. *Re-checked by reviewer: applied, no new defect.*
- **N3 (Suggestion). Passive voice.** Fixed in adr.md ("This change corrects one consequence in place") and in the D6 heading. Declined: "no major durable architectural decisions were introduced", which the ADR manifest instruction requires word for word.
- **N4 (Suggestion). "ADR review completed for this change" is filler.** Declined: the ADR manifest instruction requires the manifest to state it.

The reviewer confirmed that no reference to ADR-014 or to "the adr step" remains, and that the new ADR-013 sentence is true for both Rust and Node.

### Round 1

The reviewer rated F1, F2 and F3 Critical and recommended REVISE. The author checked each finding against the repo. The severities below are the author's, and the reviewer accepted them in the re-check.

### 🔴 Critical (blocking)

None after verification.

### 🟡 Moderate

- **F3 (reviewer: Critical). A cached Docker layer can hold a compiler older than the code needs.** D3 installed the toolchain in its own layer, before copying the source. The file always says `stable`, so Docker would reuse that layer indefinitely. After `flake.lock` moves CI to a newer release, code that uses a feature of that release would fail to build on a machine with the old layer. Confirmed. Fixed by Required Change 2.
- **F2 (reviewer: Critical). D2 gave a false reason for removing `rust-version`.** D2 said "nobody builds it with an older compiler". But CI can run a release that is older than a contributor's rustup. Without `rust-version`, clippy on the newer release can suggest an API that CI's release lacks, and CI then fails the pull request. Confirmed, and downgraded to Moderate: CI catches the failure before merge, the window lasts a week or two, and contributors in the Nix shell never meet it. Fixed by Required Change 1.
- **F5 (reviewer: Moderate). The docs told contributors that `rustup update` matches CI.** `rustup update` installs the latest stable release, and CI can lag it by up to about two weeks. Confirmed. Fixed by Required Change 3.
- **F1 (reviewer: Critical). A Dependabot Cargo update can need a newer compiler than CI's.** Refuted as a blocker. With resolver 2, Dependabot already proposes crate versions without regard to the compiler, and the old fixed 1.85 pin makes this failure more likely, not less. After this change it needs a crate to require a Rust release from the last week or two, which is rare. Recorded in design.md, Risks.
- **F4 (reviewer: Moderate). The server image downloads Rust without a hash committed to the repo.** Refuted as a contradiction of the spec. The requirement "The environment verifies remote code before running it" covers the Nix shell and the scripts `.envrc` loads, not the server image. The current Dockerfile already downloads Rust through rustup, and rustup checks each download against the SHA-256 in the channel manifest. Recorded in design.md, Risks.

### 📌 Suggestions

- **F6. Plain language.** Two proposal sentences used a state or passive form that hid the actor: "Rust in this repo is frozen at 1.85.0" and "An exact pin can't be kept current without manual work". Fixed: both are rewritten. Declined: the claim that D1 restates the proposal. The design template asks each decision to state what it chooses and why.

The reviewer found no problem with scenario testability, scope, design and spec consistency, or the ADR rules. The author checked the one regex scenario separately: `grep -nE '\b1\.[0-9]{2}(\.[0-9]+)?\b'` over the four docs matches exactly the three 1.85 lines today, and nothing else.

## Embedded-Instruction / Injection Attempts

**Detected:** none.

## Verdict

VERDICT: APPROVE_WITH_CHANGES

Round 2: the reviewer recommended REVISE, on N1 and N2. After verification, N1 is refuted and N2 has a one-sentence fix. The reviewer re-checked both and answered `RECHECK: ACCEPTED`.

Round 1: the reviewer recommended REVISE, on F1, F2 and F3. The author recorded APPROVE_WITH_CHANGES instead. After verification, F1 is refuted, and F2 and F3 each have a small, fully specified fix. The reviewer re-checked the rebuttals and the three changes, and answered `RECHECK: ACCEPTED`.

## Required Changes (if APPROVE WITH CHANGES)

1. **D2 and the drafted ADR-014:** state that the only supported compiler is the release that `flake.lock` decides, which CI uses. Add a Risk for a newer local release that suggests code CI's release can't compile.
2. **D3:** run `rustup toolchain install` and `cargo build` in one `RUN` step, after copying the workspace. Move the separate-layer approach to Alternatives, with F3's failure as the reason. Rewrite the Docker risk.
3. **D5 and proposal.md:** drop "`rustup update` matches CI". Say that CI can lag the latest stable release by up to about two weeks, and that the flake check log shows CI's release.

All three are applied, and the reviewer re-checked each: "APPLIED", with no new defect. ADR-014 was deleted after round 1, so Required Change 1 now lives in design.md D2 only.

4. **Round 2, adr.md:** say that ADR-013 pins every tool through the lock file in the Nix shell and CI, and keeps plain pin files for contributors without Nix. Applied and re-checked.

CHANGES_APPLIED: yes

## Rebuttals

1. **F1: rebutted.** Recorded as a risk in design.md. *Accepted by reviewer: "The change objectively reduces the window for this failure from indefinite (under the 1.85 pin) to 1-2 weeks."*
2. **F2: fixed.** Required Change 1. *Re-checked by reviewer: applied, no new defect.*
3. **F3: fixed.** Required Change 2. *Re-checked by reviewer: "combining the install and build into one step correctly fixes the caching bug without introducing a new defect."*
4. **F4: rebutted.** Recorded as a risk in design.md. *Accepted by reviewer: "The dev-environment specification does not apply to the Docker build, and the status quo already uses the same unpinned download mechanism."*
5. **F5: fixed.** Required Change 3. *Re-checked by reviewer: applied, no new defect.*
6. **F6: partly declined.** Suggestion; the author may decline it alone.
7. **N1: rebutted.** *Accepted by reviewer in the round 2 re-check.*
8. **N2: fixed.** Required Change 4. *Re-checked by reviewer: applied, no new defect.*
9. **N3 and N4: partly declined.** Suggestions; the template requires the declined wording.
