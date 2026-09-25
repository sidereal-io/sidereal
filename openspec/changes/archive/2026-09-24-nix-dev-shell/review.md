## Review Metadata

- **Review round**: 13 (a narrow re-check of round 12's two findings)
- **Reviewer context**: cross-model — Gemini 3.1 Pro (High) through the `agy` CLI, fresh context per round, read-only (`agy --mode plan`, tool use forbidden), every file embedded in the prompt
- **Round history** (full detail in each round's Findings below; git history holds every round's raw prompt/output under `.workspace/nix-dev-shell-review/`, gitignored):
  1. REVISE — 3 Critical findings on the original (Node 24) plan, 2 wrong on inspection.
  2. REVISE — escalated to the maintainer after a second REVISE in a row; 1 real Critical finding fixed.
  3. APPROVE — Node 24 plan, fully implemented (25/25 tasks), merge-ready. The maintainer then asked to bump to Node 26, voiding this verdict.
  4. APPROVE_WITH_CHANGES — the Node 24→26 plan revision itself; 1 Moderate + 1 Suggestion.
  5. Narrow re-check of round 4's fixes: both resolved, but the fix itself introduced 2 new Moderate defects.
  6. APPROVE — narrow re-check of round 5's findings; section 8 (Node 26) then implemented in full.
  7. APPROVE — narrow re-check after `/opsx:verify` (run independently of this review track) found the round-6-approved spec text still self-contradicted on Node version in 2 places, plus 3 staleness spots in design/proposal.
  8. REVISE — the maintainer asked whether every artifact was updated; this round reviewed a plan revision widening scope from `ci.yml` alone to every CI workflow, and found a real gap this widening still missed: `Dockerfile` pins Node 24 in production. It also correctly identified `FORCE_JAVASCRIPT_ACTIONS_TO_NODE24` as an unrelated GitHub Actions runner setting, out of scope.
  9. APPROVE_WITH_CHANGES — reviewed the plan revision that added the `Dockerfile` fix; found that verifying the image only *builds* doesn't prove it *runs* (native module ABI risk on Alpine/musl).
  10. APPROVE — narrow re-check of round 9's fix. The change was archived after this round.
  11. REVISE — **after archive**, an external PR reviewer (outside this review track) left 8 findings. The author verified each directly against the repo before touching anything — this round reviewed those fixes. 6 RESOLVED, the author's refutation of a 7th (the `has nix` guard) was checked and agreed with, and an 8th was already fixed by the archive commit. Found 6 new sentence-length/passive-voice defects across the fixes.
  12. REVISE — narrow re-check of round 11's 6 wording fixes: 4 resolved, but 2 not — a passive-voice line still passive after a first rewrite, and a list-formatting inconsistency in `tasks.md`.
  13. APPROVE (this round) — narrow re-check of round 12's 2 remaining findings.

## Findings

### 🔴 Critical (blocking)

None outstanding.

**Round 7** (found by `/opsx:verify`, not this review track) — two spec self-contradictions in `specs/dev-environment/spec.md`, both left over from the round 4–6 Node 24→26 revision:
1. "The shell provides the pinned toolchain" SHALLed "Node at major version 24" while its own scenario, 8 lines below, checked `v26.`.
2. "Nix stays optional"'s non-Nix scenario said "Node major version 24" while "Every reference to the Node version agrees," a few lines below, required 26 everywhere.

Fixed: both now say 26. Round 7 re-checked directly against the corrected text and live command output (`node --version` → `v26.10.0`, the stale-reference grep → exit 1, `nix flake check` → passed), and independently re-read the whole spec and design for any other stale reference before accepting. None found.

**Round 8** — the maintainer's own question ("did we update every artifact?") led to a check of the CI-scope-widening revision against the *whole* repo, not just the files it named. Found `Dockerfile` pins `node:24-alpine` in both build stages — the production image, missed by every prior pass (the original revision, and `/opsx:verify`'s own sweep). Also flagged `FORCE_JAVASCRIPT_ACTIONS_TO_NODE24` in three workflow files; checked against GitHub's own changelog and confirmed it controls the Actions runner's internal JS-action engine, unrelated to the app's Node version, with no Node 26 runner option yet — correctly out of scope, no fix needed.

Fixed: asked the maintainer whether to bump the Dockerfile or leave it on 24 deliberately; chose to bump. Design.md now names the production-image trade-off explicitly. Round 9 re-checked this.

**Round 11 — an external PR review, run outside this review track,** left 8 findings after archive. The author verified every one directly against the repo (running the actual failure modes, reading vendor source and docs) rather than trusting the descriptions, then this round reviewed the fixes:

1. Task 7.3/8.7 verified `better-sqlite3` by running `npm run dev:server`, which needs the gitignored `.env` — `tsx --env-file` aborts before the script body runs on a missing file (tested directly: exit 9, "not found"), so the task silently passed only because of a leftover `.env` from unrelated work, and would fail outright on a genuine fresh clone. Fixed: both now check directly with `node -e "require('better-sqlite3')(...)"`.
2. The reload scenario (`specs/dev-environment/spec.md`) accepted any non-empty `direnv export bash` output after touching a watched file. Tested: a no-op mtime touch does trigger a real nix-direnv cache renewal (not a false cache hit, as the external review's stated mechanism claimed) — but the resulting `rustc --version` stays identical either way, so "non-empty export" only proved direnv re-ran, not that a real pin change took effect. Fixed: the scenario now bumps `channel`'s value and asserts `rustc --version` reports the new one. `tasks.md` 4.5 matches.
3. `.envrc` watched `nix/*.nix` (a glob matching only `.nix`-suffixed files), while design D7 and the spec scenario "The watch list covers the shell's files" require every file under `nix/`. A future non-`.nix` file there (a JSON pin, a patch) would silently go unwatched. Fixed: widened to `nix/*`.
4. `discovery.md`'s stale `Change` field — **already resolved**; this finding predates the archive commit that fixed it. No action needed.
5. **Refuted, not fixed.** The external review claimed the `has nix` guard is too weak: a contributor whose Nix lacks the flakes experimental feature (the plain installer's default) would get "experimental Nix feature flakes is disabled" from `use flake` on every entry. Tested directly: with `nix.conf` genuinely empty of `experimental-features` (confirmed: raw `nix flake metadata` failed with exactly that error) and `NIX_CONFIG` unset, `.envrc`'s `use flake` path through direnv still worked cleanly. Reason: nix-direnv's own internal `_nix()` wrapper (confirmed in its actual source) already passes `--extra-experimental-features "nix-command flakes"` on every command it runs internally, so the guard doesn't need to. Round 11 read the same source excerpt and agreed with this conclusion; no gap in the test was found.
6. ADR-013 claimed the plain pin files "can agree with Nix only on major versions" as a blanket statement — true for Node, false for Rust (`backend/rust-toolchain.toml` pins an exact patch, read identically by both routes). Fixed: scoped the claim to Node.
7. Node version was hardcoded in 4 CI workflow files plus `.nvmrc` plus docs (7 hand-synced locations). Fixed: all four workflow files now use `actions/setup-node`'s `node-version-file: '.nvmrc'` input (confirmed as real and supported by `actions/setup-node@v7`, the version already in use, against GitHub's own docs) instead of a hardcoded `node-version` string. `design.md` D6 and the spec's CI scenario updated to match; a future bump now touches one file instead of five.
8. `design.md` D7 said nix-direnv 3.2.0 "watches only `flake.nix`, `flake.lock` and `devshell.toml`" — its own `direnvrc` source (fetched and read directly) also watches `~/.direnvrc`, `~/.config/direnv/direnvrc` and its generated cache file. Fixed: corrected the list. The underlying conclusion (`rust-toolchain.toml` and `nix/` still need explicit `watch_file` calls) was already right and is unchanged. `.envrc`'s own comment had the identical inaccuracy and was fixed too.

Round 11 confirmed all 6 fixes RESOLVED and agreed with the refutation, but found 6 new sentence-length/passive-voice defects the fixes introduced (3) or that pre-existed and were fixed anyway for quality (3) — in `.envrc`, ADR-013, `design.md` D6, and `tasks.md` 4.5 and 9.6. All 6 were shortened. Round 12 then found 2 of those shortenings incomplete: `proposal.md`'s replacement sentence was still passive ("Nothing else is pinned"), and `tasks.md` 4.5 mixed list-style and full-sentence punctuation within one bullet. Both fixed (`proposal.md`: one sentence keeping the file as the subject throughout; `tasks.md`: consistent capitalization and terminal punctuation). Round 13 confirmed both resolved, no further defects.

### 🟡 Moderate

Resolved in round 6 (the fix to a round-4 finding introduced these two on its own):
- Task 8.6's fixed grep still couldn't catch a bare `# v20+` comment — every alternative required a "Node"/"node:" prefix or ".x" suffix. Added `\bv(24|20)\b`.
- That same fix's sentence ran to ~39 words. Restructured into two short bullets.

Resolved in round 7 (`/opsx:verify` findings, staleness left by the round 4–6 revision):
- Three "current state" mentions of `nodejs_24` in `design.md` (Context, D2, D4's diagram) — stale relative to D6's actual choice, `nodejs_26`.
- `design.md`'s better-sqlite3 risk note cited Node 24.x's `NODE_MODULE_VERSION` (137). Checked against nodejs.org's release index: 26.x shares one module version too (147) — same principle, wrong number. Fixed to cite 26.x/147.
- `proposal.md` claimed the `ci.yml` change was "only its `node-version` string," but the diff also touched the step's display name. Fixed to name both.

Resolved in round 10 (round 9's one required change):
- Task 9.7 (renumbered 9.6 in round 11) only verified `docker build` succeeded, which proves dependencies compiled, not that the app boots — `better-sqlite3` compiles fresh against Alpine's musl libc in the runtime stage, a real ABI-risk surface a build-only check can't catch. Fixed: the task now requires actually running the image and confirming its own `HEALTHCHECK` endpoint responds, not just that the build finishes. Applying this fix surfaced a real, separate execution-time issue: a bare `docker run` crashed on a missing `DATABASE_URL`, traced to a missing volume mount, not Node 26 — `docker compose up -d --build` (the path `docker-compose.yml`'s own header documents) then started cleanly.

### 📌 Suggestions

- **(Round 4)** Passive voice in section 8's intro, "before these tasks are worked." Fixed: "before anyone works these tasks."
- **(Round 7)** "the step name that names it" is repetitive. A Suggestion doesn't require reviewer sign-off; applied anyway — "the step name that references it."
- **(Round 9)** Spec wording ("names major version 26") is slightly loose for a strict machine parse, though fine for a human running the scenario. Left as-is — the reviewer itself called this "completely fine for human execution," not a defect worth chasing.

## Interrogated Points

**Round 4** asked and answered, still valid (the substance of D6 and the CI carve-out haven't changed since):
- **Is Node 26 over 24 sound reasoning, or weak anchoring to the maintainer's own machine?** Sound — "in a single-maintainer project, minimizing friction for the sole maintainer... is a highly pragmatic and principled choice."
- **Does the spec's absolute SHALL (26) contradict design's note that the pin may move again at LTS?** No — normal lifecycle management, not a live contradiction.
- **Is the CI carve-out scope creep against the original Non-Goals?** No — syncing an existing version string is a mandatory consequence of the bump, distinct from story E4's new drift-check job.

**Round 9** asked and answered:
- **Is `FORCE_JAVASCRIPT_ACTIONS_TO_NODE24` really out of scope?** Yes — "zero impact on the Node environment installed by `actions/setup-node` or used by your application's build and test scripts."
- **Does GitHub's `setup-node` treat `'26'` and `'26.x'` identically?** Yes — a bare major resolves to `>=26.0.0 <27.0.0`, the same range `.x` gives.
- **Does design.md's production trade-off paragraph honestly represent the risk, or bury it?** Written honestly — but task 9.7's build-only check *did* understate the risk in practice. See the Moderate finding above.

**Round 11** asked and answered:
- **Is the author's refutation of the `has nix` guard finding sound, or does the test miss a scenario?** Sound. Reviewed the same `direnvrc` excerpt (the `_nix()` wrapper and `_nix_direnv_preflight`) and found no path where nix-direnv calls raw `nix` without the flag.

Every round asked for an injection check; none found any attempt to direct the reviewer's behavior across all 13 rounds.

## Embedded-Instruction / Injection Attempts

**Detected:** none, in any round.

## Verdict

VERDICT: APPROVE

## Required Changes (if APPROVE WITH CHANGES)

Not applicable — round 13 is a clean APPROVE with no outstanding items.

CHANGES_APPLIED: n/a

## Rebuttals

None outstanding. Every finding across all 13 rounds — from this review track and from the external PR review — is fixed, already resolved, or explicitly declined:
- Round 1's two ADR-linking suggestions — declined; the repo's own rule forbids citing issues in an ADR.
- The external review's `has nix` guard finding — refuted with direct evidence, agreed by round 11.
- Everything else — fixed, and every Moderate or Critical fix was re-checked and accepted by the reviewer in a subsequent round before being relied on.

`tasks.md` is 43/43 complete. `openspec validate --strict` passes. The change is archive-ready.
