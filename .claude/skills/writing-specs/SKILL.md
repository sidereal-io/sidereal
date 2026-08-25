---
name: writing-specs
description: Use when turning a requirement/source of demand/issue into a spec.
---

# Writing specs

Given a requirement or source of demand (a brainstorming artifact, an existing issue, a bug report):

1. Explore the repo to understand the current state of the codebase, if you haven't already. Use the project's domain glossary vocabulary throughout the spec (`CONTEXT.md`, if one exists), and respect any relevant ADRs in `docs/decisions/`.
2. Sketch out the seams at which you're going to test the feature. Existing seams should be preferred to new ones. Use the highest seam possible. If new seams are needed, propose them at the highest point you can. The fewer seams across the codebase, the better — the ideal number is one.

Check with the user that the seams match their expectations.

The spec should contain:
- **Core Objectives:** Business goals, user stories, and expected system outcomes.
- **Scope and Boundaries:** What is included and explicitly excluded from the feature.
- **Constraints and Guardrails:** Technical limitations, security rules, and architectural standards.
- **Data Contracts:** API schemas, data models, inputs, outputs, and state transitions.
- **Acceptance/Verification Criteria:** Testable conditions that prove the feature works correctly.

Per `CLAUDE.md`'s Feature & Bug Workflow, the issue body **is** the spec — there is no separate spec file in the repo tree. Write the spec into the issue body:

- New issue: `gh issue create --label type/feature,status/design --body-file .workspace/design.md`
- Revising an existing issue: pull the current body (`gh issue view <n> --json body --jq '.body' > .workspace/issue-<n>.md`), edit it in place, push it back (`gh issue edit <n> --body-file .workspace/issue-<n>.md`) — never open a new issue for a revision.

The issue stays `status/design` through this and the following `spec-critique` pass; it only moves to `status/ready` at the human approval gate.
