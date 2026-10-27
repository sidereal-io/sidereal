**Why:** Eligible Dependabot updates to `v0.x` need manual intervention because the default-branch workflow excludes their target branch.

## Story

[#445 — Enable auto-merge for maintenance-branch updates](https://github.com/sidereal-io/sidereal/issues/445).

## Decisions

| # | Question | Answer | Source |
|---|---|---|---|
| D1 | What is being fixed? | Enable maintenance-branch auto-merge through the default-branch workflow and compare each PR with its actual base. | User: “lets fix it” after the investigation and proposed fix |
| D2 | Which PR targets are supported? | `main` and `v0.x`; other branches stay excluded. | Evidence: `.github/dependabot.yml` |
| D3 | Does eligibility change? | Keep verified Dependabot metadata and the existing patch/minor policy, including its treatment of Nix updates. | Evidence: `.github/workflows/dependabot-automerge.yml`; scope of the proposed fix |
| D4 | Does push refresh need changing? | No. The workflow on `v0.x` already handles pushes to that branch. | Evidence: maintenance-branch workflow and successful push runs |
| D5 | How is the fix verified without merging a dependency PR? | Parse the workflow and execute its PR shell step with a recording `gh` stub for both target branches, including behind and current heads. | Assumption: proportional offline verification |

## Changes from the story

None.

Confirmed by user: yes — the user authorized the investigated fix in this conversation.
