## Context

<!-- Current state and constraints that shape the approach. See proposal.md for motivation; do not restate it. -->

## Goals / Non-Goals

**Goals:**
<!-- What this design aims to achieve -->

**Non-Goals:**
<!-- What is explicitly out of scope -->

## Test Seams

<!-- A seam is a point where a test can drive the code and observe the result. Test at the highest seam that still proves the behavior: fewer seams across the change is better, and one is ideal. Prefer seams the project already has. -->

- **Primary test seam:** <!-- e.g. the integration suite, a component harness -->
- **Existing seams reused:** <!-- test harnesses or test files reused -->

## Scrutiny Tier & Blast Radius

<!-- The review step reads the tier as the reviewer's effort.
     low: a narrow fix or polish; one package; no new contracts, schema or authorization.
     medium: one subsystem on established patterns; a contained blast radius.
     high: several packages; new contracts, events or migrations; authorization or trust boundaries; architecture-changing; a novel external dependency. -->
- **Tier:** <!-- low | medium | high -->
- **Blast radius:** <!-- the modules and data boundaries affected -->

## Decisions

<!-- Each technical choice, the alternatives considered, and why this one won. A decision that is costly to reverse gets a record in the ADR step; make it and name it here. -->

## Risks / Trade-offs

<!-- Known risks and trade-offs. Format: [Risk] → Mitigation -->

## Migration & Rollback

<!-- Database migrations, data backfills, rollback strategy. State "N/A" if nothing changes shape. -->

## Open Questions

<!-- Unknowns that can wait without changing the specs, the approach or the tasks. The tasks step reads this section. Omit the section if there are none. -->
