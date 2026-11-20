## Story

#498 ci-on-mise — every CI job installs its tools from mise.lock, https://github.com/sidereal-io/sidereal/issues/498

## Decisions

| # | Question | Answer | Source |
|---|---|---|---|
| D1 | Which `ci.yml` jobs does this story move off Nix, and what stays out? | All six jobs: `compose`, `server`, `web`, `openspec`, `e2e` and `lets-spec`. Out: renaming recipes to mise tasks (#499) and the `nix` workflow (#502). | Evidence: #498 packet, Scope |
| D2 | How does each job get its tools? | It installs them from `mise.lock` (for example with `jdx/mise-action`) and has no Nix step. Each command runs as `mise exec -- <command>`. | Evidence: #498 packet, Scope |
| D3 | How does the cargo cache step run its Rust commands? | Through mise, so it installs no Rust of its own. | Evidence: #498 packet, Scope |
| D4 | Does the `compose` job follow the rule, or is it exempt? | It follows the rule: it installs from `mise.lock` and runs `mise exec -- just check-compose`. The `ci` spec says Docker and jq come from the runner. | Assumption |
| D5 | The `lets-spec` job runs `mise exec -- lets sync --check`, which is not a `just` recipe. Is it exempt from the one-recipe rule? | Yes, from the one-recipe rule only, not from installing through mise. It checks the files Let's Spec writes, not project code, and `just check` does not run it. A new recipe would only grow the list that #499 replaces. | Assumption |
| D6 | The `e2e` job runs several steps, not one recipe. Is it exempt from the one-recipe rule? | Yes. Its steps keep their order and each runs as `mise exec -- <command>`. The `ci` spec names the reasons already in `ci.yml`: Chromium's system libraries need root, and the server must be built before Playwright's 300-second startup window. | Assumption |
| D7 | The `ci` spec still says which files start the `server` and `web` jobs, and that a docs-only pull request runs neither. That has been false since #430 removed the path filters. Does this story fix it? | No. Those requirements stay as they are. The user files the drift as a separate bug with `lets-shape`. | Assumption |
| D8 | Which `ci` spec requirements change? | "Every CI job runs one recipe inside the development shell" says mise instead of the Nix shell, and names the exempt jobs. "Each CI job runs its own check recipe" names `check-compose` for the `compose` job, and its scenarios expect `mise exec -- just <recipe>`. | Evidence: #498 packet, Scope; `openspec/specs/ci/spec.md` |
| D9 | After this change, `AGENTS.md` (lines 40-41) and `CONTRIBUTING.md` (lines 188-189 and 245) would still say the `ci` workflow takes its Rust release from `flake.lock`. Does this change correct them? | Yes, only the sentences about where CI gets its tools: they say the `ci` workflow installs from `mise.lock` and rustup decides the Rust release. The wider docs rewrite stays in #502. | Assumption |

## Changes from the story

- Added: the `e2e` job, which the packet does not name, moves to mise too (D6). It is a `ci.yml` job, so "every job" covers it.
- Split out: the path-filter drift in the `ci` spec (D7) becomes a separate bug, for the user to file with `lets-shape`. lets-go reports it to them; this change files nothing.

lets-go confirmed this ledger (D1-D9 and the changes above) on 2026-11-19, under the user's standing instruction to stop in the brainstorm only when a question has no recommendation. The user did not read or decide this ledger in person. D4-D9 are lets-go's assumptions. The marker below is `yes` because the schema needs it to start the proposal.

Confirmed by user: yes
