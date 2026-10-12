# OpenSpec Workflow Improvements & Alignment

This document outlines the findings, gaps, and required changes to enhance Sidereal's OpenSpec workflow by incorporating the battle-tested disciplines from `writing-specs`, `spec-critique`, `writing-plans`, and `executing-plans`.

It provides both the **exact schema instructions** (for `openspec/schemas/spec-driven-review/schema.yaml`) and the **exact file templates** (under `templates/`).

---

## 1. Executive Summary & Core Deficiencies in Current Workflow

While Sidereal pioneered the multi-model `spec-driven-review` schema (`proposal` → `specs` → `design` → `adr` → `review` → `tasks`), several structural gaps exist when comparing it to mature plan-and-execute workflows:

1. **Sequential Task Bottleneck & Missing Sizing/TDD Rhythm:**
   - Tasks are currently a flat list of checkboxes (`- [ ] X.Y`).
   - There is no model for parallel execution or explicit dependency tracking between workstreams (e.g. backend Rust engine vs. web frontend UI).
   - Tasks lack sizing heuristics (5–12 tasks per plan slice), TDD rhythm, and complexity ratings (`[easy]`, `[medium]`, `[complex]`) for subagent dispatch.

2. **Missing Highest Seam Principle & Scrutiny Tiering in Design:**
   - Designs lack an explicit requirement to declare the *highest viable architectural seam* for testing.
   - Without this, implementations risk defaulting to brittle, low-level unit mocks rather than integration test harnesses at major domain boundaries.
   - Blast radius is not formally classified into scrutiny tiers (`low`, `medium`, `high`).

3. **Adversarial Defect Checklist Not Standardized in Review:**
   - The review gate does not mandate the comprehensive hunt list from `spec-critique` (repo verification, missing edge cases/failure modes, cross-tenant/auth boundaries, state/migration hazards, and simpler alternatives).

4. **Lack of "Tick = Evidence" & Deviations Ledger in Apply:**
   - The apply phase does not enforce that completed tasks include inline verification evidence (`✓ command passed`).
   - There is no formal ledger for logging execution deviations or hard stop triggers when hitting unexpected blockers.

5. **Unstructured Explore Mode (Missing Brainstorming Gate):**
   - The `openspec-explore` skill does not force an explicit decision on whether brainstorming is necessary, risking diving directly into proposals when UX, domain terms, or architectural paths are still ambiguous.

6. **Persona Readability:**
   - `openspec/discovery.md` and `AGENTS.md` lack punchy 1-line taglines on personas, making quick persona grounding during discovery less immediate.

---

## 2. Exact Schema Instructions (`schema.yaml`)

Below are the exact `instruction:` blocks for `openspec/schemas/spec-driven-review/schema.yaml`.

### A. Design Artifact Instruction (`id: design`)

```yaml
  - id: design
    generates: design.md
    description: Technical approach, architectural decisions, and alternatives
    template: design.md
    instruction: |
      Create the technical design document.

      Guidelines:
      - Focus on how the change will be implemented, not why it's needed
        (that's in the proposal).
      - Document decisions made and alternatives considered, with rationale.
      - Highlight risks, trade-offs, and mitigation strategies.
      - Identify the primary test seam at the highest viable architectural point
        (the Highest Seam Principle).
      - Classify the scrutiny tier based on blast radius.

      Sections:
      - **Context**: Only the current state and constraints needed to explain the approach.
        Reference the proposal for motivation instead of restating it.
      - **Goals / Non-Goals**: What this design achieves and explicitly excludes.
        Don't restate the proposal's scope - add only design-level boundaries.
      - **Test Seams (Highest Seam Principle)**: Sketch out the seams at which the feature
        will be tested. Existing seams should be preferred to new ones. Propose test seams
        at the highest possible architectural point (the fewer seams across the codebase,
        the better — ideal is one).
      - **Scrutiny Tier & Blast Radius**: Classify blast radius:
        * `low`: Narrow fix or polish; single package; no new contracts, schema, or authz.
        * `medium`: Standard single-subsystem slice on established patterns; contained blast radius.
        * `high`: Multi-package; new contracts/events/migrations; authz or trust boundaries;
          architecture-changing (new-ADR territory); novel external dependency.
      - **Decisions**: Key technical choices with rationale (why X over Y?). Include alternatives
        considered for each decision.
      - **Risks / Trade-offs**: Known limitations, things that could go wrong. Format: [Risk] → Mitigation.
      - **Migration & Rollback**: Steps to deploy, database migrations, rollback strategy (if applicable).
      - **Open Questions**: Unknowns that can safely be answered later without changing the specs,
        the approach, or the task breakdown. Omit if none.
    requires:
      - proposal
```

---

### B. Review Artifact Instruction (`id: review`)

```yaml
  - id: review
    generates: review.md
    description: Adversarial multi-model peer review gate for proposal, specs, design, and adr
    template: review.md
    instruction: |
      Conduct an independent, adversarial peer review of the proposal, specs, design, and adr.
      The reviewer MUST run using an independent model family (e.g. GPT/Codex reviewing Claude,
      or Claude reviewing Gemini) in read-only mode.

      The reviewer critiques; it does not rewrite or implement.

      Hunt specifically for:
      - **Repo Verification**: Check all assumptions against live code in the repo (verify, don't assume).
      - **Missing Edge Cases & Failure Modes**: Concrete failure inputs, partial failures, timeouts,
        scale hazards, service outages.
      - **Security & Trust Boundaries**: Authz gaps, cross-tenant leaks, trust boundaries crossed
        without validation.
      - **State & Migration Hazards**: Database schema changes, data loss, backward incompatibility,
        missing rollback strategy.
      - **Simpler Alternatives**: Simpler designs that deliver identical guarantees with less code/abstraction.
      - **Plain language in every artifact**: Proposal, design, specs, adr. Flag sentences > 30 words,
        passive voice, or filler narrative that restates motivation.
      - **Scenario Testability**: Every delta spec scenario's THEN condition must be mechanically assertable.
      - **Highest Seam**: Verify tests are positioned at the highest viable architectural seam.
      - **Scope vs. proposal**: Creep or gold-plating beyond proposal scope.
      - **Design/spec contradictions**: Design choices conflicting with spec requirements.

      Output ranked findings, most severe first (blocker / major / minor), citing evidence from the repo.
      Triaging: Mark each finding CONFIRMED, REFUTED, or OUT-OF-SCOPE.

      The canonical verdict line MUST be written to review.md:
      `VERDICT: APPROVE`
      or `VERDICT: APPROVE_WITH_CHANGES` (with `CHANGES_APPLIED: yes` after revisions),
      or `VERDICT: REVISE`.
      Downstream artifacts (tasks.md) MUST NOT proceed until approved.
    requires:
      - proposal
      - specs
      - design
      - adr
```

---

### C. Tasks Artifact Instruction (`id: tasks`)

```yaml
  - id: tasks
    generates: tasks.md
    description: Implementation checklist with trackable tasks and parallel execution support
    template: tasks.md
    instruction: |
      Create the task list that breaks down the implementation work.

      Before writing tasks, check design.md for Open Questions. If any of them
      would change what gets built, resolve them with the user first - do not
      bake an unstated assumption into the task list.

      **IMPORTANT: Follow the template below exactly.** The apply phase parses
      checkbox format to track progress. Tasks not using `- [ ]` won't be tracked.

      Sizing & Task Rhythm (from writing-plans):
      - **5–12 tasks total**: If larger, recommend slicing into separate stories/changes.
      - **TDD rhythm**: Group work to write failing tests depicting intent → run to verify failure →
        minimal code → run to verify pass.
      - **Complexity ratings**: Tag each task with `[easy]`, `[medium]`, or `[complex]` to guide
        model/subagent assignment.
      - **Hard Stop**: If an ADR-tier decision is required during task planning, STOP and return to design.

      Parallel Execution & Dependency Guidelines:
      - Structure phases:
        * Phase 1: Shared contracts, types, migrations (Sequential prerequisite)
        * Phase 2+: Independent workstreams labeled with `[depends-on: <phase>]`
        * Final Phase: Integration and end-to-end verification `[depends-on: <all>]`
      - Annotate parallelizable tasks with `[parallel]` within workstreams so orchestrators
        can dispatch subagents concurrently.
      - Each task MUST be a checkbox: `- [ ] X.Y [tags] Task description (verify: <command>)`.
      - Each task MUST state how to verify completion (e.g. `cargo test`, `pnpm test`, `pnpm check`).
    requires:
      - review
```

---

### D. Apply Operation Instruction (`apply:`)

```yaml
apply:
  requires: [ tasks ]
  tracks: tasks.md
  instruction: |
    Execution discipline (from executing-plans):
    - **Tick = Evidence**: A checkbox is ticked `- [x]` only with its verification evidence
      recorded inline:
      `- [x] 1.1 Add audio chunk parser (✓ cargo test parser passed)`.
      A ticked box without evidence is invalid. Never tick ahead; commit as you go.
    - **Atomic commits**: Commit per completed task with real Conventional Commit types
      (`feat:`, `fix:`, `test:`, `refactor:`).
    - **Parallel dispatch**: Orchestrating agent may launch concurrent subagents for independent
      workstreams whose `[depends-on]` prerequisites are complete.
    - **Deviations ledger**: Append any minor deviations into a `## Deviations Ledger` section
      in tasks.md. Stop and surface material ones (changed guarantees, authz, schema).
    - **Stop triggers**: STOP immediately when hitting unexpected blockers, repeated test failures,
      or if an ADR-level fork arises.
```

---

## 3. Exact Templates (`templates/`)

### A. `templates/design.md`

```markdown
## Context

<!-- Current state and constraints that shape the approach. See proposal.md for motivation - don't restate it -->

## Goals / Non-Goals

**Goals:**
<!-- What this design aims to achieve -->

**Non-Goals:**
<!-- What is explicitly out of scope -->

## Test Seams

<!-- Highest Seam Principle: Propose testing at the highest viable architectural point (ideal is one seam across the slice). Prefer existing seams over new ones. -->

- **Primary test seam:** <!-- e.g. integration suite / component harness -->
- **Existing seams reused:** <!-- list existing test harnesses or test files reused -->

## Scrutiny Tier & Blast Radius

<!-- low (narrow fix, single package) | medium (single subsystem, known patterns) | high (multi-package, contracts, migrations, authz, ADR-territory) -->
- **Tier:** <!-- low | medium | high -->
- **Blast radius:** <!-- brief description of affected modules and data boundaries -->

## Decisions

<!-- Key design decisions with rationale and alternatives considered. If an ADR-tier fork arises, STOP and document in adr.md. -->

## Risks / Trade-offs

<!-- Known risks and trade-offs. Format: [Risk] → Mitigation -->

## Migration & Rollback

<!-- Database migrations, data backfills, rollback strategy. State "N/A" if no schema changes. -->
```

---

### B. `templates/tasks.md`

```markdown
## 1. Setup & Shared Contracts (Sequential)

<!-- 5–12 tasks total across the plan. Slice if larger. -->
<!-- TDD rhythm: write failing test depicting intent → verify failure → implement minimal code → verify pass. -->
<!-- Tag complexity [easy|medium|complex] to guide subagent/model assignment. -->

- [ ] 1.1 [easy] <!-- Task description with verification command (verify: <command>) -->
- [ ] 1.2 <!-- Task description with verification command -->

## 2. Workstream A: Core Engine [depends-on: 1]

<!-- Tasks marked [parallel] can run concurrently with other independent streams -->
- [ ] 2.1 [parallel] [medium] <!-- Task description (verify: <command>) -->
- [ ] 2.2 [parallel] [medium] <!-- Task description (verify: <command>) -->

## 3. Workstream B: Surface UI / Client [depends-on: 1]

<!-- Tasks marked [parallel] can run concurrently with other independent streams -->
- [ ] 3.1 [parallel] [medium] <!-- Task description (verify: <command>) -->
- [ ] 3.2 [parallel] [easy] <!-- Task description (verify: <command>) -->

## 4. Integration & System Verification [depends-on: 2, 3]

- [ ] 4.1 [complex] <!-- Full system verification test (verify: <command>) -->

## Deviations Ledger

<!-- Tick = Evidence discipline: A box is ticked only with its inline verification evidence. -->
<!-- Append dated deviations below during apply. Stop and surface material divergences. -->
```

---

## 4. Skill & Workflow Integration

### A. `openspec-explore` Brainstorming Evaluation Gate

In `.agents/skills/openspec-explore/SKILL.md` (and `.claude/skills/openspec-explore/SKILL.md`), place this gate directly above `## Planning a Change`:

```markdown
## Brainstorming Evaluation Gate

At the start of exploring any new feature, open problem, or requirement, you MUST make an explicit decision: **Is a brainstorm necessary?**

Evaluate:
1. Is the requirement open-ended or ambiguous?
2. Are there multiple viable UX, product, or architectural paths?
3. Are new domain terms or entities emerging that are not in the glossary?
4. Does this significantly touch core journeys in `openspec/discovery.md`?

- **If YES to any of the above:**
  Explicitly invoke the `brainstorming` skill.
  Ground the exploration in `openspec/discovery.md`, relevant ADRs, and codebase reality.
  Drive `grilling` to ask one question at a time to converge on scope.
  Use `domain-modeling` to update glossary and architecture maps.
  Only once the brainstorm converges, proceed to propose the change with `openspec new change <slug>`.

- **If NO (e.g. straightforward bug fix, mechanical refactor, well-specified task):**
  State concisely why brainstorming is unnecessary (e.g. "Scope is concrete and well-bounded: [reason]").
  Proceed directly to discovery and proposal drafting.
```

### B. `AGENTS.md` Lifecycle Sections

Update steps 4 & 5 under `OpenSpec Lifecycle` in `AGENTS.md`:

```markdown
4. **Task Breakdown (`tasks.md`):**
   - Sizing: Target **5–12 tasks total**. Slice into separate changes if larger.
   - TDD Rhythm: Structure tasks into failing test → verify failure → minimal code → verify pass.
   - Tag complexity: `[easy]`, `[medium]`, `[complex]` to guide subagent/model assignment.
   - **Parallel Execution Support:** Group tasks into workstreams with dependency markers (`[depends-on: <phase>]`) and tag concurrent tasks with `[parallel]`. Independent streams (e.g. backend Rust engine vs web UI) can be executed concurrently by subagents.
   - Move project Status to `Ready`.

5. **Apply (Implementation):**
   - **Tick = Evidence:** A checkbox is ticked `- [x]` only with its verification evidence inline (`✓ <command> passed`). Never tick ahead.
   - **Atomic commits:** Commit per unit of work with real Conventional Commit types (`feat:`, `fix:`, `test:`, `refactor:`).
   - **Deviations ledger:** Append deviations to `## Deviations Ledger` in `tasks.md`. Stop and surface material ones (authz, schema, guarantees).
   - **Stop triggers:** Stop immediately on repeated test failures or unexpected blockers.
```
