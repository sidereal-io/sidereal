---
name: domain-modeling
description: Build and sharpen the project's domain model. Use when the user wants to pin down domain terminology or a ubiquitous language, record an architectural decision, or when another skill needs to maintain the domain model.
---

# Domain Modeling

Actively build and sharpen the project's domain model as you design. This is the *active* discipline — challenging terms, inventing edge-case scenarios, and writing the glossary and decisions down the moment they crystallise. (Merely *reading* `CONTEXT.md` for vocabulary is not this skill — that's a one-line habit any skill can do. This skill is for when you're changing the model, not just consuming it.)

## File structure

This repo already has a domain reference — `docs/architecture/README.md` owns the architectural map and the load-bearing concepts (Asset, AssetVersion, Collection, Selector, Lineage, Processing Goal, Operation Run, and the core/domain-pack split). Read it first; extend it rather than starting a parallel glossary.

For terminology that doesn't belong in that architectural map (feature-local vocabulary, in-progress brainstorming language not yet promoted), use a root `CONTEXT.md`:

```
/
├── CONTEXT.md
├── docs/
│   ├── architecture/README.md   ← the architectural map and glossary of load-bearing concepts
│   └── decisions/                ← ADRs — see CLAUDE.md's Decision Records section
└── apps/, packages/, tools/
```

Create `CONTEXT.md` lazily — only when you have a term that doesn't fit `docs/architecture/README.md`'s scope. If a term turns out to be load-bearing (used across packages, referenced by an ADR), promote it into `docs/architecture/README.md`'s Core concepts section instead of leaving it in `CONTEXT.md`.

## During the session

### Challenge against the glossary

When the user uses a term that conflicts with the existing language in `docs/architecture/README.md` or `CONTEXT.md`, call it out immediately. "The architecture doc defines 'Collection' as X, but you seem to mean Y — which is it?"

### Sharpen fuzzy language

When the user uses vague or overloaded terms, propose a precise canonical term. "You're saying 'image' — do you mean the Asset or a specific AssetVersion? Those are different things here."

### Discuss concrete scenarios

When domain relationships are being discussed, stress-test them with specific scenarios. Invent scenarios that probe edge cases and force the user to be precise about the boundaries between concepts.

### Cross-reference with code

When the user states how something works, check whether the code agrees. If you find a contradiction, surface it: "The code treats a rename as a path event with no new AssetVersion, but you just said renames create lineage — which is right?"

### Update the glossary inline

When a term is resolved, update `CONTEXT.md` (or `docs/architecture/README.md`'s Core concepts, if load-bearing) right there. Don't batch these up — capture them as they happen. Use the format in [CONTEXT-FORMAT.md](./CONTEXT-FORMAT.md) for `CONTEXT.md` entries.

`CONTEXT.md` should be totally devoid of implementation details. Do not treat it as a spec, a scratch pad, or a repository for implementation decisions. It is a glossary and nothing else.

### Offer ADRs sparingly

Follow `CLAUDE.md`'s Decision Records section — the ADR/Design fact/Scope tiers, the lifecycle, and the template at `docs/decisions/ADR-000-template.md`. Only offer to create an ADR when all three are true:

1. **Hard to reverse** — the cost of changing your mind later is meaningful
2. **Surprising without context** — a future reader will wonder "why did they do it this way?"
3. **The result of a real trade-off** — there were genuine alternatives and you picked one for specific reasons

If any of the three is missing, skip the ADR — it's either a one-line Design fact under `docs/architecture/README.md`'s Design facts section, or not durable enough to record at all. Before drafting a new ADR, scan `docs/decisions/` for a **Proposed** one this decision would have to assume an answer to — if it exists, stop and resolve that first (per `CLAUDE.md`'s "Open ADRs block design" rule).
