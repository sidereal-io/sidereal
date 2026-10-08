---
id: adrs-adr005
date: 2026-10-05
status: accepted
title: 'ADR005: v2 Visual Design System'
description: Architecture Decision Record (ADR) for how v2 screens look and where that is defined — a DESIGN.md source of truth with tiered tokens, mirrored into code and a Penpot design library.
---

# ADR-005: v2 Visual Design System

## Context

The design has to work in three places at once: in code (`web/`), in a design
tool (Penpot), and in the context that coding agents read. If more than one of
those can be edited first, they drift apart.

Sidereal is self-hosted, often on a home network with no internet access, and
is used at night near a telescope.

## Decision

v2 screens follow a **dark, instrument-like design defined in `DESIGN.md`** at
the repo root, which is the single source of truth for every token value and
usage rule.

- **Look:** the Obsidian Deep Space direction, with violet-tinted near-black
  surfaces, Inter for the interface, and Atkinson Hyperlegible Mono for data.
- **Rules:** violet only for the primary action, selection, focus, and live
  work; one glow on screen at a time; a status shown only inside a badge; depth from
  surface steps and 1px borders, not shadows; square image corners; text
  contrast of at least 4.5:1; dark only.
- **Format:** `DESIGN.md` follows the open DESIGN.md specification (YAML front
  matter for values, prose for rules). Values outside the specification's keys,
  such as individual type values, opacity, and blur, go in an `x-sidereal`
  extension that only Sidereal's token generator reads.
- **Tokens in three tiers:** primitives, semantic, and component. The design
  starts with primitives only. A semantic or component token is added by the
  first component that needs it, in the same pull request. A primitive exists
  only if `DESIGN.md` names a use for it.
- **Code and design follow `DESIGN.md`.** `tokens.css` is generated from it and
  checked against it; it is the only generated file. Before an agent works in
  the Penpot library, it copies the tokens from `DESIGN.md` into Penpot over the
  Penpot MCP server. Penpot's tokens are never edited first.
- **Fonts are self-hosted** with the web shell.
- **Nothing visual is ported from the v0.10.x interface.**

## Consequences

- Agents and contributors read one file to know how a screen should look, and
  the same file is what gets injected into design and planning prompts.
- The token set stays small. Most roles don't exist as tokens until a component
  needs them, so early components may each add one or two semantic tokens.
- Every token change touches `DESIGN.md` and `tokens.css`. The generator and
  the checks make that cheap. Penpot gets the change when an agent next copies
  the tokens, so it may lag behind `DESIGN.md` until someone works there. No code
  reads Penpot's tokens, so the lag affects only design work.
- The DESIGN.md specification is still at version alpha. If it changes, the
  front matter may need to follow. The `x-sidereal` extension is ignored by the
  specification's own exporters, so Sidereal's generator must read it directly.
- If a screen story ports a component from the v0.10.x interface, the port takes
  every visual value from the tokens, not from the old styles.
- Light mode, a night-vision theme, and mobile layouts are out of scope. The
  token names leave room for a light mode later.

## Alternatives Considered

### Alternative 1: Neutral grays, system UI font, IBM Plex Mono
- **Pros**: no font files to ship; clear, strict usage rules; nothing to retune
  for light mode later.
- **Cons**: a generic look with nothing specific to astrophotography; gives up
  the tinted surfaces and data colors that make telemetry easy to scan.
- **Why not**: its rules were kept, but the look didn't carry the product's
  character.

### Alternative 2: Obsidian Deep Space as written, without usage rules
- **Pros**: the most complete visual reference, already expressed as a
  DESIGN.md file.
- **Cons**: three accent colors and several glows with no rule for when to use
  them; its muted text color and primary button both fail contrast.
- **Why not**: without rules, each screen would spend the accents differently.

### Alternative 3: Align with the v0.10.x theme
- **Pros**: one look across both interfaces during the transition.
- **Cons**: carries over styling that was never designed as a system, into an
  interface with a different information model.
- **Why not**: v2 is a new design. Whether any v0.10.x component is ported at
  all is decided screen by screen, not here.

### Alternative 4: Penpot as the source of truth
- **Pros**: designers work where tokens live; Penpot already speaks the W3C
  design-token format.
- **Cons**: every token change needs Penpot open; agents can't propose a change
  in a pull request; adds a build step between the design tool and the repo.
- **Why not**: the repo is where agents and reviewers work, so the source of
  truth belongs there.

### Alternative 5: A complete component library up front
- **Pros**: every screen starts with finished parts.
- **Cons**: builds and maintains parts no screen needs yet, and fixes decisions
  before the screens that would test them exist.
- **Why not**: components should be built when a screen first needs them.
