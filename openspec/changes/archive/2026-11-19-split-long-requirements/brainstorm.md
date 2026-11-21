**Why:** OpenSpec 2.1.1 warns on 61 requirements over 500 characters, and the strict gate fails on any warning, so the specs must be split before #506 can move the CLI to OpenSpec 2.

## Story

#505 split-long-requirements — every spec passes OpenSpec 2's strict check: https://github.com/sidereal-io/sidereal/issues/505

## Decisions

| # | Question | Answer | Source |
|---|---|---|---|
| D1 | How does the change deliver the split? | Edit the 11 main specs in `openspec/specs/` directly as build tasks, and set `skip_specs: true` in `.openspec.yaml`, because no behavior changes. Archive only moves the change folder. | Assumption |
| D2 | May an existing requirement heading change? | No. Each heading stays on the part that keeps the core rule; split-off parts become new requirements with new headings, so later changes in epic #496 still find their targets. | Assumption |
| D3 | How much may the wording change? | Move sentences as they are. Reword only where a split sentence must stand on its own. Every rule, condition and exact format survives. | Assumption |
| D4 | What proves the change is done? | `npx @fission-ai/openspec@2.1.1 validate --all --strict --no-interactive` passes; `just check` passes on OpenSpec 1.13.2; and a script in `.workspace/` shows each spec keeps every scenario name it had before. | Assumption |
| D5 | Which requirements are in scope? | The 61 that OpenSpec 2.1.1 flags on this branch: instance-access 17, compose-instance 11, ci 6, postgres-foundation 6, external-postgres 4; design-tokens, dev-commands, managed-storage, security-events and web-shell 3 each; dev-environment 2. This matches the issue. | Evidence: `npx @fission-ai/openspec@2.1.1 validate --all --strict --no-interactive --json` on this branch, 2026-11-19 |
| D6 | Is the #497 dependency met? | Yes. This branch stacks on `mise-dev-env` (#504), which carries #497's `dev-environment` requirements, so the split cannot conflict with them. | Evidence: `git log` of `split-long-requirements` and `mise-dev-env` |

## Changes from the story

None.

Confirmed by user: yes
