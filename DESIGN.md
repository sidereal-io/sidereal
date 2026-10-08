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
    letter-spacing-060: 0.06em
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
  border-width:
    border-width-1: 1px
  size:
    size-32: 32px
    size-1280: 1280px
  semantic: {}
components: {}
---

# Sidereal design system

## Overview

Sidereal is a system of record for astrophotographers. Its screens hold raw
frames, calibration masters, finished images, and dense technical metadata.

## How to use this file

- **This file is the source of truth** for every token value and usage rule in
  the web interface (`web/`). Code and the Penpot library follow it.
- **The front matter holds the values.** It follows the
  [DESIGN.md specification](https://github.com/google-labs-code/design.md)
- **A token's CSS name is its key with the `sr` prefix:** `obsidian-950`
  becomes `--sr-obsidian-950`, and `spacing-8` becomes `--sr-spacing-8`.
- **To change a token,** edit this file first. In the same pull request, run
  `pnpm tokens` in `web/` to regenerate `tokens.css`. Never change a token in
  Penpot first.
- **Penpot is the design workspace.** Explore there freely, but a design change
  is only real once it lands here. The Penpot file is **Sidereal Design System**
  (ID `3e981c57-46d6-803d-8008-bedef465c177`): it holds a copy of the primitive
  tokens and, as they are designed, the components. Agents reach it through the
  Penpot MCP server and plugin, which a person connects to the file. Before an
  agent works there, it copies the tokens from this file into Penpot, so Penpot
  may lag behind this file until then.
  - **Pages:** `Primitives` shows the primitive tokens, `Components` holds the
    components, and `Explorations` holds design options a person is comparing.
  - **Token sets:** `primitives` is a copy of this file's primitive tokens.
    Only the copy step changes it; never edit it by hand. A value being tried
    out goes in a set named `explore-<topic>`, one set per exploration, never
    in `primitives`.

Tokens come in three tiers: **primitives, semantic tokens, and component tokens**.

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
| A hairline border | `obsidian-800` | Always `border-width-1`. |
| Body text | `obsidian-100` | A warm off-white, easier on the eye than pure white. |
| Secondary text | `obsidian-400` | |
| Muted text that must still be read: placeholders, metadata keys, timestamps | `obsidian-500` | |
| Disabled text and decorative separators | `obsidian-600` | Never for text a person needs to read. |
| A filled primary action with a text label | `violet-600`, label `white` | One primary action per view. |
| A focus ring, selection outline, or live-work glow | `violet-500` | No text on top of it. |
| Telemetry values: temperature, connection, focal length | `cyan-500` | Data only, never an action. |
| Exposure and calibration notices | `amber-500` | Data only, never an action. |
| Status: success, warning, error, info | `emerald-400`, `amber-500`, `red-500`, `cyan-500` | Show a status only inside a badge. Outside badges, `amber-500` and `cyan-500` carry only the data uses above. |
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

- **Inter** (`font-family-sans`) is the interface face: navigation, headings,
  body copy, dialogs.
- **Atkinson Hyperlegible Mono** (`font-family-mono`) is the data face: FITS
  header keys and values, coordinates, exposure values, filenames, hashes,
  timestamps, and logs. It keeps lookalike characters distinct (`0` and `O`,
  `1`, `l`, and `I`, `5` and `S`).
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
| mono-lg | mono | `font-size-14` | `line-height-20` | `font-weight-500` | |
| mono-md | mono | `font-size-12` | `line-height-18` | `font-weight-400` | |
| mono-sm (chips, tags) | mono | `font-size-11` | `line-height-16` | `font-weight-400` | |
| label-caps (chips, tags) | mono | `font-size-10` | `line-height-14` | `font-weight-600` | `letter-spacing-060` |

Mono text has no letter spacing: Atkinson Hyperlegible Mono is already widely
spaced. Only all-caps labels are tracked out, by `letter-spacing-060`.

## Layout

### Two layout families, one token set

Every screen uses one of two layout families. Choose by the screen's main
content:

- **Gallery family:** use it when images are the main content, as on Home,
  Albums, the asset viewer, and the sky map. First-run setup also uses it. Images
  run edge to edge, panels are kept to a minimum, and controls float over the
  images.
- **Admin family:** use it when text, tables, or forms are the main content, as on
  Pipeline, Targets, Equipment, Locations, and Settings. It has a fixed sidebar, a
  content column no wider than `size-1280`, and dense tables.

### Grid and spacing

- All spacing sits on a 4px grid.
- Desktop screens use a 12-column grid with `spacing-32` margins and
  `spacing-24` gutters.
- Table rows are `size-32` tall, for dense data.

| Spacing | Use |
|---|---|
| `spacing-4` | Gaps between badges, key-value pairs, and chips |
| `spacing-8` | Small gaps inside controls and lists |
| `spacing-12` | Card padding and the gap between form fields |
| `spacing-20` | Space between major panes |
| `spacing-24` | Desktop column gutter |
| `spacing-32` | Desktop page margin, and space between catalog groups |

## Elevation & Depth

Sidereal shows depth with surface colors and borders. A dark shadow barely shows
on near-black surfaces, so don't add shadows.

- **Resting depth comes from layering:** `obsidian-950` for the page,
  `obsidian-900` for panels, and `obsidian-850` for popovers. Each layer is edged
  with a `border-width-1` border.
- **A glow marks something selected, active, live, or hovered.** A glow is a
  color at an opacity with a blur, with no offset and no spread. Use only the
  glows in the table below.
- **Only one glow is on screen at a time.** A hover glow shows only while the
  pointer is on the control, and while it shows, it replaces any other glow. When
  nothing is hovered, a selected item's glow takes priority over an active
  instrument's glow.

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
- **Borders always use `border-width-1`.**

## Components

There are no components yet. Each component is built when a screen first needs
it, styled only through tokens, and designed in Penpot before it is built. When a
component needs a role this file doesn't name, it adds a semantic token; see
[How tokens grow](#how-tokens-grow).

## Do's and Don'ts

- **Do** use `violet` only for the primary action, the current selection, focus,
  and live work.
- **Do** show a status (success, warning, error, info) only inside a badge.
  Outside badges, `amber-500` and `cyan-500` carry only their data uses.
- **Do** show depth with surface steps and borders.
- **Do** put every value a person must read exactly in the mono face.
- **Don't** use a raw value in a component. If a component needs a value that
  has no token, name its use in this file and add the token first.
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
  `x-sidereal` for individual type values, opacity, blur, border width, and
  sizes.

### Tier 2 · Semantic (alias)

A semantic token names a role or function, such as "the color of a primary
action" or "subtle text". It is an alias: it points to a primitive, so changing
what a role looks like means changing one reference.

- **Naming:** the category, then the context, then the modifier:
  `color-action-primary` is a color, for an action, of the primary kind.
- **Value:** always a reference to a primitive, never a raw value. For example,
  `color-action-primary` → `{colors.violet-600}`. A reference into `x-sidereal`
  works only through Sidereal's token generator.
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
- **Value:** each visual property is a reference to a semantic token or a
  primitive, never a raw value.
- **When to add one:** only when a component must look different from every
  semantic token. The pull request says why in one line.
- **Who uses it:** only the component it is named for.
- **Where it lives:** the specification's `components` key. The token name is
  the key, and each visual property sits beneath it, the way the specification
  structures components. For example, `button-primary-hover` with the property
  `backgroundColor: "{colors.violet-500}"`. Its CSS name adds the property:
  `--sr-button-primary-hover-background-color`.

### Changing any token

Edit this file first. In the same pull request, run `pnpm tokens` in `web/` to
regenerate `tokens.css`. Penpot gets the change when an agent next copies the
tokens into it.

## Candidates

These values come from the Obsidian Deep Space design this system started from.
None has an approved use in this file yet. Each becomes a primitive when a
component gives it one.

| Candidate | Value | Source use | Note |
|---|---|---|---|
| Frame type: light | `#38BDF8` | LIGHT frame badge | |
| Frame type: dark | `#818CF8` | DARK frame badge | |
| Frame type: flat | `#F59E0B` | FLAT frame badge | Same value as `amber-500`, which already means "warning". A flat-frame color needs its own value. |
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
