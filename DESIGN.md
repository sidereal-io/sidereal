---
version: alpha
name: Sidereal
description: >-
  Dark, instrument-like interface for an astrophotography system of record.
  Tokens here are Tier 1 primitives only; semantic and component tokens are
  added when a component first needs them.
colors:
  obsidian-950: "#0B0A0F"
  obsidian-900: "#121118"
  obsidian-850: "#1A1823"
  obsidian-800: "#272435"
  obsidian-700: "#3F3A53"
  obsidian-600: "#635F6F"
  obsidian-500: "#837F90"
  obsidian-400: "#9E9AA7"
  obsidian-100: "#F5F4EF"
  violet-500: "#8B5CF6"
  violet-600: "#7C3AED"
  cyan-500: "#06B6D4"
  amber-500: "#F59E0B"
  red-500: "#EF4444"
  emerald-400: "#34D399"
  white: "#FFFFFF"
rounded:
  radius-4: 4px
  radius-8: 8px
  radius-full: 9999px
spacing:
  spacing-4: 4px
  spacing-8: 8px
  spacing-12: 12px
  spacing-20: 20px
  spacing-24: 24px
  spacing-32: 32px
x-sidereal:
  css-prefix: sr
  font-family:
    font-family-sans: 'Inter, system-ui, -apple-system, "Segoe UI", sans-serif'
    font-family-mono: '"Atkinson Hyperlegible Mono", ui-monospace, "SF Mono", Menlo, monospace'
  font-weight:
    font-weight-400: 400
    font-weight-500: 500
    font-weight-600: 600
  font-size:
    font-size-10: 10px
    font-size-11: 11px
    font-size-12: 12px
    font-size-14: 14px
    font-size-16: 16px
    font-size-20: 20px
    font-size-30: 30px
  line-height:
    line-height-14: 14px
    line-height-16: 16px
    line-height-18: 18px
    line-height-20: 20px
    line-height-24: 24px
    line-height-28: 28px
    line-height-38: 38px
  letter-spacing:
    letter-spacing-n015: -0.015em
    letter-spacing-n010: -0.01em
    letter-spacing-010: 0.01em
    letter-spacing-020: 0.02em
    letter-spacing-080: 0.08em
  opacity:
    opacity-4: 0.04
    opacity-20: 0.2
    opacity-25: 0.25
    opacity-45: 0.45
  blur:
    blur-8: 8px
    blur-10: 10px
    blur-12: 12px
    blur-14: 14px
  semantic: {}
components: {}
---

# Sidereal design system

## Overview

Sidereal is a system of record for astrophotographers. Its screens hold raw
frames, calibration masters, finished images, and dense technical metadata. The
interface is built for three things:

- **Dark adaptation.** People use Sidereal at night, near a telescope. Surfaces
  are near-black with a faint violet tint, and nothing glows without a reason.
- **Instrument-grade precision.** Thin 1px borders, tabular numbers, and a
  monospace face for every value a person has to read exactly.
- **Photographic depth.** Images are the content. Interface chrome stays quiet
  around them.

### How to use this file

- **This file is the source of truth** for every token value and usage rule in
  the v2 web interface (`web/`). Code and the Penpot library follow it.
- **The front matter holds the values.** It follows the
  [DESIGN.md specification](https://github.com/google-labs-code/design.md).
  Values the specification has no key for (type, opacity, blur, and later
  semantic tokens) live under `x-sidereal`. Component tokens go in the
  specification's own `components` key.
- **A token's CSS name is its key with the `sr` prefix:** `obsidian-950`
  becomes `--sr-obsidian-950`, and `spacing-8` becomes `--sr-spacing-8`.
- **To change a token,** edit this file first. In the same pull request,
  regenerate `tokens.css` and the design-token JSON from it. After merge, import
  that JSON into Penpot. Never change a token in Penpot first.
- **Penpot is the design workspace.** Explore there freely, but a design change
  is only real once it lands here. The Penpot file is **Sidereal Design System**
  (ID `3e981c57-46d6-803d-8008-bedef465c177`): it holds the imported primitive
  tokens and, as they are designed, the components. Agents reach it through the
  Penpot plugin, which a person connects to the file.

Tokens come in three tiers: primitives, semantic tokens, and component tokens.
Only primitives exist today. [How tokens grow](#how-tokens-grow) explains each
tier and when to add to it.

## Colors

The palette has one tinted neutral family, two violets for interaction, three
data colors, a success green, and white. The usage table below is guidance for
the first components; each row becomes a semantic token when a component first
needs it.

| When a component needs… | Use | Rule |
|---|---|---|
| The page background | `obsidian-950` | |
| A raised panel, card, or table row | `obsidian-900` | |
| A popover, dialog, or menu | `obsidian-850` | Edge it with `obsidian-700`. |
| A hairline border | `obsidian-800` | Always 1px. |
| Body text | `obsidian-100` | A warm off-white, easier on the eye than pure white. |
| Secondary text | `obsidian-400` | |
| Muted text that must still be read: placeholders, metadata keys, timestamps | `obsidian-500` | |
| Disabled text and decorative separators | `obsidian-600` | Never for text a person needs to read. |
| A filled primary action with a text label | `violet-600`, label `white` | One primary action per view. |
| A focus ring, selection outline, or live-work glow | `violet-500` | No text on top of it. |
| Telemetry values: temperature, connection, focal length | `cyan-500` | Data only, never an action. |
| Exposure and calibration notices | `amber-500` | Data only, never an action. |
| Status: success, warning, error, info | `emerald-400`, `amber-500`, `red-500`, `cyan-500` | In badges only. |
| A ghost button's hover tint | `white` at `opacity-4` | |

### Contrast

Text needs a contrast ratio of at least 4.5:1 against its background. Large text
(24px and up, or 19px bold and up) and meaningful graphics need 3:1. Measured on
the darkest and lightest surfaces:

| Pair | On `obsidian-950` | On `obsidian-900` | On `obsidian-850` |
|---|---|---|---|
| `obsidian-100` text | 17.9 | 17.0 | — |
| `obsidian-400` text | 7.2 | 6.8 | 6.4 |
| `obsidian-500` text | 5.1 | 4.8 | 4.5 |
| `obsidian-600` text | 3.2 (fails) | 3.0 (fails) | 2.8 (fails) |
| `violet-500` text | 4.7 | 4.4 (fails) | — |

`white` on `violet-600` is 5.7:1. `white` on `violet-500` is only 4.2:1, which is
why filled actions use `violet-600`.

## Typography

- **Inter** is the interface face: navigation, headings, body copy, dialogs.
- **Atkinson Hyperlegible Mono** is the data face: FITS header keys and values,
  coordinates, exposure values, filenames, hashes, timestamps, and logs. It keeps
  lookalike characters distinct (`0` and `O`, `1`, `l`, and `I`, `5` and `S`).
- **Both fonts are self-hosted** with the web shell. Sidereal often runs on a
  home network with no internet access, so it never loads fonts from a font
  service.
- **Columns of numbers use tabular figures,** so digits line up.

These styles are guidance for the first text components. Each becomes a semantic
token when a component first needs it.

| Style | Family | Size | Line height | Weight | Letter spacing |
|---|---|---|---|---|---|
| headline-lg (page title) | sans | `font-size-30` | `line-height-38` | `font-weight-600` | `letter-spacing-n015` |
| headline-md | sans | `font-size-20` | `line-height-28` | `font-weight-600` | `letter-spacing-n010` |
| headline-sm | sans | `font-size-16` | `line-height-24` | `font-weight-600` | |
| body-lg | sans | `font-size-16` | `line-height-24` | `font-weight-400` | |
| body-md | sans | `font-size-14` | `line-height-20` | `font-weight-400` | |
| body-sm | sans | `font-size-12` | `line-height-18` | `font-weight-400` | |
| mono-lg | mono | `font-size-14` | `line-height-20` | `font-weight-500` | `letter-spacing-020` |
| mono-md | mono | `font-size-12` | `line-height-18` | `font-weight-400` | `letter-spacing-010` |
| mono-sm (chips, tags) | mono | `font-size-11` | `line-height-16` | `font-weight-400` | `letter-spacing-020` |
| label-caps (chips, tags) | mono | `font-size-10` | `line-height-14` | `font-weight-600` | `letter-spacing-080` |

The mono letter spacing was set for a different monospace face. Check it against
Atkinson Hyperlegible Mono before the first mono style becomes a token.

## Layout

### Two layout families, one token set

- **Gallery family:** if a screen's main content is pixels, such as Home, Albums,
  the asset viewer, the sky map, or first-run setup. Images run edge to edge,
  chrome is minimal, and controls float over the canvas.
- **Admin family:** if a screen's main content is text, tables, or forms, such as
  Pipeline, Targets, Equipment, Locations, or Settings. A fixed sidebar, a
  content column with a maximum width, and dense tables.

### Grid and spacing

- All spacing sits on a 4px grid.
- Desktop screens use a 12-column grid with `spacing-32` margins and
  `spacing-24` gutters.
- Tables use compact 32px rows.

| Spacing | Use |
|---|---|
| `spacing-4` | Gaps between badges, key-value pairs, and chips |
| `spacing-8` | Small gaps inside controls and lists |
| `spacing-12` | Card padding and the gap between form fields |
| `spacing-20` | Space between major panes |
| `spacing-24` | Desktop column gutter |
| `spacing-32` | Desktop page margin, and space between catalog groups |

## Elevation & Depth

Shadows don't work on near-black surfaces, so Sidereal doesn't use them.

- **Resting depth comes from layering:** `obsidian-950` for the page,
  `obsidian-900` for panels, and `obsidian-850` for popovers. Each layer is edged
  with a 1px border.
- **A glow marks something selected, active, or live.** A glow is a color at an
  opacity with a blur, with no offset and no spread.
- **Only one glow is on screen at a time.**

| Glow | Color | Opacity | Blur |
|---|---|---|---|
| Selected item | `violet-500` | `opacity-25` | `blur-12` |
| Active instrument | `cyan-500` | `opacity-20` | `blur-10` |
| Primary button, hover | `violet-500` | `opacity-45` | `blur-14` |
| Secondary button, hover (its border also turns `cyan-500`) | `cyan-500` | `opacity-20` | `blur-8` |

## Shapes

- **Images have square corners.** Rounding would crop the frame edges, where
  vignetting and stacking artifacts show.
- **Controls** (buttons, inputs, badges, tags) use `radius-4`.
- **Containers** (panels, popovers, floating control bars) use `radius-8`.
- **Pills and dots** (coordinate chips, status dots) use `radius-full`.
- **Borders are always 1px.**

## Components

There are no components yet. Each component is built when a screen first needs
it, styled only through tokens, and designed in Penpot before it is built. When a
component needs a role this file doesn't name, it adds a semantic token; see
[How tokens grow](#how-tokens-grow).

## Do's and Don'ts

- **Do** use `violet` only for the primary action, the current selection, focus,
  and live work.
- **Do** keep status colors inside badges.
- **Do** show depth with surface steps and borders.
- **Do** put every value a person must read exactly in the mono face.
- **Don't** use a raw value in a component when a token exists.
- **Don't** add a shadow, or a second glow.
- **Don't** round image corners.
- **Don't** use `obsidian-600` for text anyone needs to read.
- **Don't** copy styles from the v0.10.x interface.

## How tokens grow

Tokens come in three tiers. Each tier builds on the one below it. Only Tier 1
exists today; Tiers 2 and 3 grow one token at a time, when a component first
needs them.

| Tier | Holds | Name structure | Examples |
|---|---|---|---|
| 1 · Primitive (global) | Raw values with no context | `{category}-{scale}` or `{category}-{weight}` | `violet-600`, `spacing-8`, `font-weight-500` |
| 2 · Semantic (alias) | A role or function | `{category}-{context}-{modifier}` | `color-action-primary`, `color-text-subtle` |
| 3 · Component | One element of one component | `{component}-{element}-{state}` | `button-primary-hover` |

### Tier 1 · Primitives (global)

A primitive is a raw value, such as a color, a size, or a weight. It says what a
value **is**, never what it is **for**: `violet-600` is a shade of violet, not
"the button color".

- **Naming:** colors use a 50–950 scale from light to dark (`obsidian-950` is
  the darkest). Sizes are named by their value in pixels (`spacing-8` is 8px,
  `font-size-12` is 12px). Where a number doesn't fit, the step is a word
  (`radius-full`, `font-family-sans`). A negative value starts with `n`
  (`letter-spacing-n015` is −0.015em).
- **Value:** always a raw value, such as `#7C3AED` or `8px`.
- **When to add one:** only when this file names a concrete use for it. Values
  with no use yet wait in [Candidates](#candidates).
- **Who uses it:** semantic tokens. A component may use a primitive directly
  only until a semantic token for that role exists.
- **Where it lives:** the `colors`, `spacing`, and `rounded` keys, and under
  `x-sidereal` for type, opacity, and blur.

### Tier 2 · Semantic (alias)

A semantic token names a role or function, such as "the color of a primary
action" or "subtle text". It is an alias: it points to a primitive, so changing
what a role looks like means changing one reference.

- **Naming:** the category, then the context, then the modifier:
  `color-action-primary` is a color, for an action, of the primary kind.
- **Value:** always a reference to a primitive, never a raw value. For example,
  `color-action-primary` → `{colors.violet-600}`.
- **When to add one:** when a component first needs the role, in the same pull
  request. Start from the usage tables in this file: each row there is a
  semantic token waiting to be created.
- **Who uses it:** components. Once a semantic token exists, components use it
  instead of the primitive it points to.
- **Where it lives:** `x-sidereal.semantic`.

### Tier 3 · Component

A component token is tied to one element of one component. It exists only when
no semantic token fits, for example when a button's hover state needs a value
that no role describes.

- **Naming:** the component, then the element, then the state:
  `button-primary-hover` is the button's primary variant, on hover.
- **Value:** a reference to a semantic token or a primitive, never a raw value.
- **When to add one:** only when a component must look different from every
  semantic token. The pull request says why in one line.
- **Who uses it:** only the component it is named for.
- **Where it lives:** the specification's `components` key.

### Changing any token

Edit this file first. In the same pull request, regenerate `tokens.css` and the
design-token JSON. After merge, import the JSON into Penpot.

## Candidates

These values appear in the source design but have no use yet. Each becomes a
primitive when a component gives it one.

| Candidate | Value | Source use | Note |
|---|---|---|---|
| Frame type: light | `#38BDF8` | LIGHT frame badge | |
| Frame type: dark | `#818CF8` | DARK frame badge | |
| Frame type: flat | `#F59E0B` | FLAT frame badge | Same as `amber-500`, which already means "warning". Choose a distinct color. |
| Frame type: bias | `#A78BFA` | BIAS frame badge | |
| Frame type: final | `#EC4899` | FINAL or master badge | |
| `font-size-24`, `line-height-32` | 24px / 32px | Mobile headline | Mobile layouts are out of scope. |
| `font-size-40`, `line-height-48` | 40px / 48px | Display headline | No screen uses it yet. |
| `spacing-16` | 16px | Mobile margin and gutter | Mobile layouts are out of scope. |
| `radius-12` | 12px | Full-screen modal frames | |

## Non-goals

- Light mode. It may come later, for the admin family only. Gallery screens stay
  dark.
- A night-vision (red) theme.
- Mobile layouts.
- Porting visual styles from the v0.10.x interface.
