# OpenSpec Workflow Improvements & Alignment

This document outlines the findings, gaps, and required changes to enhance Sidereal's OpenSpec workflow by incorporating the battle-tested disciplines from `writing-specs`, `spec-critique`, `writing-plans`, and `executing-plans`.

---

## 1. Executive Summary & Core Deficiencies in Current Workflow

While Sidereal pioneered the multi-model `spec-driven-review` schema (`proposal` → `specs` → `design` → `adr` → `review` → `tasks`), several structural gaps exist when comparing it to mature plan-and-execute workflows:

1. **Sequential Task Bottleneck:**
   - Tasks are currently a flat list of checkboxes (`- [ ] X.Y`).
   - There is no model for parallel execution or explicit dependency tracking between workstreams (e.g. backend Rust engine vs. web frontend UI).
   - Tasks lack sizing heuristics (5–12 tasks per plan slice) and complexity ratings (`[easy]`, `[medium]`, `[complex]`) for subagent dispatch.

2. **Missing Highest Seam Principle in Design:**
   - Designs lack an explicit requirement to declare the *highest viable architectural seam* for testing.
   - Without this, implementations risk defaulting to brittle, low-level unit mocks rather than integration test harnesses at major domain boundaries.

3. **Missing Scrutiny Tiering in Design:**
   - Blast radius is not formally classified into scrutiny tiers (`low`, `medium`, `high`), making it difficult for the multi-model review to scale its reasoning effort appropriately.

4. **Adversarial Defect Checklist Not Standardized in Review:**
   - The review gate does not mandate the comprehensive hunt list from `spec-critique` (repo verification, missing edge cases/failure modes, cross-tenant/auth boundaries, state/migration hazards, and simpler alternatives).

5. **Lack of "Tick = Evidence" & Deviations Ledger in Apply:**
   - The apply phase does not enforce that completed tasks include inline verification evidence (`✓ command passed`).
   - There is no formal ledger for logging execution deviations or hard stop triggers when hitting unexpected blockers.

6. **Unstructured Explore Mode (Missing Brainstorming Gate):**
   - The `openspec-explore` skill does not force an explicit decision on whether brainstorming is necessary, risking diving directly into proposals when UX, domain terms, or architectural paths are still ambiguous.

7. **Persona Readability:**
   - `openspec/discovery.md` and `AGENTS.md` lack punchy 1-line taglines on personas, making quick persona grounding during discovery less immediate.

---

## 2. Required Changes by Component

### A. Schema Updates: `openspec/schemas/spec-driven-review/schema.yaml`

#### 1. Design Artifact (`id: design`)
Add requirements for **Test Seams** (Highest Seam Principle) and **Scrutiny Tier & Blast Radius**:
- **Test Seams:** Require identifying the primary test seam at the highest viable architectural point, preferring existing test harnesses.
- **Scrutiny Tier:** Classify change as `low` (narrow fix), `medium` (single subsystem), or `high` (multi-package, contracts, migrations, authz, or ADR-level).

#### 2. Review Artifact (`id: review`)
Enrich review checks with the adversarial checklist from `spec-critique`:
- **Repo Verification:** Verify claims and existing system state against live code.
- **Missing Edge Cases & Failure Modes:** Concrete failure inputs, partial failures, timeout/scale hazards.
- **Security & Trust Boundaries:** Authz gaps, trust boundaries crossed without validation.
- **State & Migration Hazards:** Data loss, backward incompatibility, missing rollback strategy.
- **Simpler Alternatives:** Simpler architectures delivering identical guarantees with less code.
- **Scenario Testability:** Mechanically assertable `THEN` conditions in delta specs.
- **Highest Seam:** Verification that tests are positioned at the highest viable seam.

#### 3. Tasks Artifact (`id: tasks`)
Incorporate `writing-plans` sizing and parallel workstream support:
- **5–12 Tasks Total:** If larger, slice into separate changes.
- **TDD Rhythm:** Write failing test depicting intent → verify failure → minimal code → verify pass.
- **Complexity Ratings:** Tag tasks with `[easy]`, `[medium]`, or `[complex]`.
- **Parallel Workstreams:** Support `[depends-on: <phase>]` on headings and `[parallel]` tags on concurrent tasks.
- **Hard Stop:** Stop immediately if an ADR-tier decision is required during task breakdown.

#### 4. Apply Operation (`apply:`)
Incorporate `executing-plans` execution discipline:
- **Tick = Evidence:** `- [x]` is valid only with inline verification command and output:
  `- [x] 1.1 Add parser (✓ cargo test parser passed)`.
- **Atomic Commits:** Conventional commits per task (`feat:`, `fix:`, `test:`, `refactor:`).
- **Parallel Dispatch:** Concurrent subagents may execute independent workstreams whose dependencies are satisfied.
- **Deviations Ledger:** Append dated deviations to `## Deviations Ledger`.
- **Stop Triggers:** Hard stop on unexpected blockers or repeated test failures.

---

### B. Template Updates

#### 1. `openspec/schemas/spec-driven-review/templates/design.md`
Add sections:
```markdown
## Test Seams

<!-- Highest Seam Principle: Propose testing at the highest viable architectural point. Prefer existing seams over new ones. -->
- **Primary test seam:** <!-- e.g. integration suite / component harness -->
- **Existing seams reused:** <!-- list test harnesses reused -->

## Scrutiny Tier & Blast Radius

<!-- low (narrow fix) | medium (single subsystem) | high (multi-package, contracts, migrations, authz, ADR-territory) -->
- **Tier:** <!-- low | medium | high -->
- **Blast radius:** <!-- affected modules and boundaries -->
```

#### 2. `openspec/schemas/spec-driven-review/templates/tasks.md`
Update template to support phases, workstreams, dependencies, and deviations:
```markdown
## 1. Setup & Shared Contracts (Sequential)

<!-- 5–12 tasks total. TDD rhythm: failing test → verify failure → minimal code → verify pass. Tag complexity [easy|medium|complex]. -->
- [ ] 1.1 [easy] <!-- Task description (verify: <command>) -->
- [ ] 1.2 <!-- Task description (verify: <command>) -->

## 2. Workstream A: Core Engine [depends-on: 1]

<!-- Tasks marked [parallel] can run concurrently with other independent streams -->
- [ ] 2.1 [parallel] [medium] <!-- Task description (verify: <command>) -->

## 3. Workstream B: Surface UI / Client [depends-on: 1]

<!-- Tasks marked [parallel] can run concurrently with other independent streams -->
- [ ] 3.1 [parallel] [medium] <!-- Task description (verify: <command>) -->

## 4. Integration & System Verification [depends-on: 2, 3]

- [ ] 4.1 [complex] <!-- Full system verification test (verify: <command>) -->

## Deviations Ledger

<!-- Tick = Evidence discipline: A box is ticked only with its inline verification evidence. -->
<!-- Append dated deviations below during apply. Stop and surface material divergences. -->
```

---

### C. Skill Updates: `openspec-explore`

In `.agents/skills/openspec-explore/SKILL.md` (and `.claude/skills/openspec-explore/SKILL.md`), add an explicit **Brainstorming Evaluation Gate**:

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

---

### D. Documentation Updates: `openspec/discovery.md` & `AGENTS.md`

1. **Short Persona Summaries:**
   Add 1-line taglines directly below persona headings in `openspec/discovery.md` and `AGENTS.md` so that agents and operators have instant context.
2. **Git Workflow Lifecycle in `AGENTS.md`:**
   - Update Step 4 (Task Breakdown) with parallel execution markers and 5–12 task sizing.
   - Update Step 5 (Apply) with the Tick = Evidence rule and deviations tracking.
   - Confirm `AGENTS.md` serves as the sole source of truth across all agent harnesses.
