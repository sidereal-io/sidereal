# design-tokens Specification

## Purpose
The web shell's design tokens come from `DESIGN.md`, the source of truth for how screens look. The web gate fails when the generated tokens, `DESIGN.md` itself, or its contrast claims drift.

## Requirements

### Requirement: The token file holds every primitive from DESIGN.md

`web/src/styles/tokens.css` SHALL define one CSS custom property for each primitive token and each semantic token in `DESIGN.md`'s front matter. The primitives are the entries under `colors`, `rounded`, and `spacing`, and under each group in `x-sidereal` except `css-prefix` and `semantic`. The semantic tokens are the entries under `x-sidereal.semantic`.

The generator SHALL write this file. Contributors SHALL NOT edit it by hand.

#### Scenario: One property per primitive

- **WHEN** a tester counts the primitive tokens in `DESIGN.md`'s front matter and the custom properties in `web/src/styles/tokens.css` whose value does not start with `var(`
- **THEN** both counts are equal
- **AND** every primitive's key appears in exactly one property name

#### Scenario: One property per semantic token

- **WHEN** a tester counts the entries under `x-sidereal.semantic` in `DESIGN.md` and the custom properties in `web/src/styles/tokens.css` whose value starts with `var(`
- **THEN** both counts are equal
- **AND** every semantic token's key appears in exactly one property name

#### Scenario: A new token

- **WHEN** a tester adds `spacing-16: 16px` under `spacing` in `DESIGN.md`
- **AND** runs the generator
- **THEN** `web/src/styles/tokens.css` defines `--sr-spacing-16` with the value `16px`

### Requirement: Each property's name and value come from its token

Each property's name SHALL be `--`, then the value of `x-sidereal.css-prefix`, then `-`, then the token's key. A primitive's property value SHALL be the token's value as `DESIGN.md` writes it. A semantic token's property value SHALL be `var(` and the property name of the primitive it references, then `)`. The file SHALL define no other custom property.

#### Scenario: A color token

- **WHEN** a reviewer reads `web/src/styles/tokens.css`
- **THEN** it defines `--sr-obsidian-950` with the value `#0B0A0F`

#### Scenario: A token from the extension

- **WHEN** a reviewer reads `web/src/styles/tokens.css`
- **THEN** it defines `--sr-font-size-14` with the value `14px`
- **AND** it defines `--sr-opacity-45` with the value `0.45`

#### Scenario: A semantic alias

- **WHEN** `DESIGN.md` holds `color-action-primary: "{colors.violet-600}"` under `x-sidereal.semantic`
- **AND** a tester runs the generator
- **THEN** `web/src/styles/tokens.css` defines `--sr-color-action-primary` with the value `var(--sr-violet-600)`

### Requirement: The generator refuses tokens it cannot write yet

The generator SHALL exit with a non-zero status and write no file when `DESIGN.md` holds any token it cannot write. These are:

- a semantic token whose value is not exactly `{<group>.<key>}`, naming an existing primitive, with nothing before or after it;
- any entry under `components`;
- any entry under `typography`, or a `colors`, `rounded`, or `spacing` key that is not a map;
- two tokens with the same key, in any groups.

Its error message SHALL name the token.

#### Scenario: A semantic token

- **WHEN** a tester adds `color-action-primary: "#7C3AED"` under `x-sidereal.semantic` in `DESIGN.md`
- **AND** runs the generator
- **THEN** the generator exits with a non-zero status
- **AND** its output contains `color-action-primary`
- **AND** `git diff --exit-code web/src/styles/tokens.css` exits with status 0

#### Scenario: A semantic token that names no primitive

- **WHEN** a tester adds `color-action-primary: "{colors.violet-650}"` under `x-sidereal.semantic` in `DESIGN.md`, where `colors` has no `violet-650`
- **AND** runs the generator
- **THEN** the generator exits with a non-zero status
- **AND** its output contains `color-action-primary`

#### Scenario: A semantic value with text after its reference

- **WHEN** a tester changes `color-action-primary` under `x-sidereal.semantic` in `DESIGN.md` to `"{colors.violet-600}; } body {"`
- **AND** runs the generator
- **THEN** the generator exits with a non-zero status
- **AND** its output contains `color-action-primary`
- **AND** `git diff --exit-code web/src/styles/tokens.css` exits with status 0

#### Scenario: A component token

- **WHEN** a tester adds `button-primary-hover: { backgroundColor: "{colors.violet-500}" }` under `components` in `DESIGN.md`
- **AND** runs the generator
- **THEN** the generator exits with a non-zero status
- **AND** its output contains `button-primary-hover`

#### Scenario: A duplicate key

- **WHEN** a tester adds `spacing-4: 4px` under `x-sidereal.size` in `DESIGN.md`
- **AND** runs the generator
- **THEN** the generator exits with a non-zero status
- **AND** its output contains `spacing-4`

### Requirement: The generator refuses a key or value that could escape its CSS

The generator SHALL exit with a non-zero status and write no file when `DESIGN.md` holds any of these:

- a key that holds anything other than lowercase letters, digits, and hyphens;
- an `x-sidereal.css-prefix` that is missing, or holds anything other than lowercase letters;
- a primitive value that holds a line break, `;`, `{`, `}`, `<`, `\`, or the text `url(`.

Its error message SHALL name the token.

#### Scenario: A value that would escape its property

- **WHEN** a tester changes `size-1280` under `x-sidereal.size` in `DESIGN.md` to `"1px; } body { background: url(https://x.invalid) }"`
- **AND** runs the generator
- **THEN** the generator exits with a non-zero status
- **AND** its output contains `size-1280`
- **AND** `git diff --exit-code web/src/styles/tokens.css` exits with status 0

#### Scenario: A prefix that would escape its block

- **WHEN** a tester changes `x-sidereal.css-prefix` in `DESIGN.md` to `"sr; } body { color: red } /*"`
- **AND** runs the generator
- **THEN** the generator exits with a non-zero status
- **AND** its output contains `css-prefix`
- **AND** `git diff --exit-code web/src/styles/tokens.css` exits with status 0

### Requirement: A color or pixel value fits its group's form

The generator SHALL exit with a non-zero status and write no file when `DESIGN.md` holds a value in these groups that does not fit its group's form. Its error message SHALL name the token.

| Group | Form of each value | Example |
|---|---|---|
| `colors` | a hex color: `#` and 6 hex digits | `#0B0A0F` |
| `rounded`, `spacing`, and `x-sidereal`'s `font-size`, `line-height`, `blur`, `border-width`, and `size` | a whole number of pixels | `14px` |

#### Scenario: A spacing value in another unit

- **WHEN** a tester changes `spacing-4` under `spacing` in `DESIGN.md` to `4em`
- **AND** runs the generator
- **THEN** the generator exits with a non-zero status
- **AND** its output contains `spacing-4`
- **AND** `git diff --exit-code web/src/styles/tokens.css` exits with status 0

### Requirement: A number value fits its group's form

The generator SHALL exit with a non-zero status and write no file when `DESIGN.md` holds a value in these groups that does not fit its group's form. Its error message SHALL name the token.

| Group | Form of each value | Example |
|---|---|---|
| `x-sidereal.letter-spacing` | a decimal number of `em`, which may be negative | `-0.015em` |
| `x-sidereal.opacity` | a number from 0 to 1 | `0.45` |
| `x-sidereal.font-weight` | a whole number from 1 to 1000 | `600` |

#### Scenario: An opacity above 1

- **WHEN** a tester changes `opacity-45` under `x-sidereal.opacity` in `DESIGN.md` to `1.5`
- **AND** runs the generator
- **THEN** the generator exits with a non-zero status
- **AND** its output contains `opacity-45`
- **AND** `git diff --exit-code web/src/styles/tokens.css` exits with status 0

### Requirement: A font family value fits its group's form

The generator SHALL exit with a non-zero status and write no file when `DESIGN.md` holds a value in these groups that does not fit its group's form. Its error message SHALL name the token.

| Group | Form of each value | Example |
|---|---|---|
| `x-sidereal.font-family` | a comma-separated list. Each item is a name in double quotes, or words that each start with a letter or a hyphen. No item may be a CSS-wide keyword, such as `inherit` | `Inter, -apple-system, "Segoe UI", sans-serif` |

#### Scenario: A value that does not fit its group

- **WHEN** a tester changes `font-family-sans` under `x-sidereal.font-family` in `DESIGN.md` to `12px`
- **AND** runs the generator
- **THEN** the generator exits with a non-zero status
- **AND** its output contains `font-family-sans`
- **AND** `git diff --exit-code web/src/styles/tokens.css` exits with status 0

### Requirement: The generator refuses an x-sidereal group with no form

The generator SHALL exit with a non-zero status and write no file when `DESIGN.md` holds an `x-sidereal` group that none of the form tables names. Its error message SHALL name the token.

#### Scenario: An unknown group

- **WHEN** a tester adds a group `shadow` holding `shadow-1: 1px` under `x-sidereal` in `DESIGN.md`
- **AND** runs the generator
- **THEN** the generator exits with a non-zero status
- **AND** its output contains `shadow-1`
- **AND** `git diff --exit-code web/src/styles/tokens.css` exits with status 0

### Requirement: Stylesheets use only tokens that exist

The unit tests SHALL fail when a stylesheet or TypeScript file under `web/src/` uses, through `var()`, a custom property that no stylesheet under `web/src/` defines. `tokens.css` counts as one of those stylesheets. The test SHALL check every `var()` name, whatever its prefix. The test's output SHALL name the property and the file that uses it.

#### Scenario: The prefix changed, but the stylesheets did not

- **WHEN** a tester changes `x-sidereal.css-prefix` in `DESIGN.md` to `foo`, runs the generator, and runs `mise run check-web`
- **THEN** the command exits with a non-zero status
- **AND** its output contains `--sr-obsidian-950`

#### Scenario: A token in use was removed

- **WHEN** a tester removes `obsidian-900`, which no semantic token references, from `DESIGN.md`, runs the generator, and runs `mise run check-web`
- **THEN** the command exits with a non-zero status
- **AND** its output contains `--sr-obsidian-900`

### Requirement: The web gate catches a stale token file

The token drift check SHALL compare `web/src/styles/tokens.css` with the output the generator would write from `DESIGN.md`. It SHALL exit with a non-zero status when they differ, and print the command that regenerates the file. It SHALL NOT change any file.

#### Scenario: DESIGN.md changed, but the token file did not

- **WHEN** a tester changes `obsidian-950` to `#0C0B10` in `DESIGN.md`, without running the generator
- **AND** runs `mise run check-web`
- **THEN** the command exits with a non-zero status
- **AND** its output names the command that regenerates the token file
- **AND** `git diff --exit-code web/src/styles/tokens.css` exits with status 0

#### Scenario: The token file was edited by hand

- **WHEN** a tester changes the value of `--sr-obsidian-950` in `web/src/styles/tokens.css`
- **AND** runs `mise run check-web`
- **THEN** the command exits with a non-zero status

#### Scenario: The token file was regenerated

- **WHEN** a tester changes `obsidian-950` in `DESIGN.md`, runs the generator, and runs `mise run check-web`
- **THEN** the token drift check passes

### Requirement: The web gate lints DESIGN.md

The web gate SHALL run the DESIGN.md specification's linter on `DESIGN.md`. Any linter error SHALL fail the gate. A linter warning SHALL fail the gate unless it is one of the three known warnings, matched by rule and path:

| Rule | Path |
|---|---|
| `missing-primary` | `colors` |
| `missing-typography` | `typography` |
| `token-like-ignored` | `x-sidereal` |

When a known warning stops appearing, the gate SHALL still pass.

#### Scenario: DESIGN.md has only the known warnings

- **WHEN** a contributor runs `mise run check-web` on an unchanged checkout
- **THEN** the linter step passes

#### Scenario: A new warning appears

- **WHEN** a tester adds a top-level key `x-other` with the entry `thing-4: 4px` to `DESIGN.md`'s front matter
- **AND** runs `mise run check-web`
- **THEN** the command exits with a non-zero status
- **AND** its output contains `x-other`

#### Scenario: A color value is not a color

- **WHEN** a tester changes `obsidian-950` to `"#GGGGGG"` in `DESIGN.md`
- **AND** runs `mise run check-web`
- **THEN** the command exits with a non-zero status

### Requirement: The web gate checks DESIGN.md's contrast claims

The unit tests SHALL check every cell of the table in `DESIGN.md`'s Contrast section. The first cell of each row names a text color token. Each column header names a background color token. For each cell that holds a number, the test SHALL compute the WCAG 2 contrast ratio between the two tokens' values in the front matter.

A cell that holds `—` claims nothing, and the test SHALL skip it.

#### Scenario: The table is correct

- **WHEN** a contributor runs `mise run check-web` on an unchanged checkout
- **THEN** the contrast test passes

### Requirement: A contrast claim matches the computed ratio

The contrast test SHALL fail when:

- the computed ratio, rounded to one decimal place, differs from the number in the cell;
- the cell is marked `(fails)`, but the computed ratio is at least 4.5;
- the cell is not marked `(fails)`, but the computed ratio is below 4.5.

#### Scenario: A color changed, but its claims did not

- **WHEN** a tester changes `obsidian-500` to `#6F6B7C` in `DESIGN.md`
- **AND** runs `mise run check-web`
- **THEN** the command exits with a non-zero status

#### Scenario: A claimed ratio is wrong

- **WHEN** a tester changes the `obsidian-400` cell on `obsidian-950` from `7.2` to `7.5`
- **AND** runs `mise run check-web`
- **THEN** the command exits with a non-zero status

#### Scenario: A failing pair is not marked

- **WHEN** a tester removes `(fails)` from the `obsidian-600` cell on `obsidian-950`
- **AND** runs `mise run check-web`
- **THEN** the command exits with a non-zero status

### Requirement: A contrast table the test cannot read fails the test

The contrast test SHALL fail when:

- the table is missing, has no background columns, or has no rows;
- the table names a token that is not a color in the front matter;
- a cell holds anything other than a number with one decimal place, that number followed by `(fails)`, or `—`.

The test SHALL read a row that leaves out its outer pipes, as Markdown allows, and SHALL read `DESIGN.md` the same way with either line ending.

#### Scenario: A cell holds no claim the test can read

- **WHEN** a tester changes the `obsidian-500` cell on `obsidian-850` from `4.5` to `TBD`
- **AND** runs `mise run check-web`
- **THEN** the command exits with a non-zero status

#### Scenario: The table has no rows

- **WHEN** a tester deletes every row of the contrast table in `DESIGN.md` and keeps its header
- **AND** runs `mise run check-web`
- **THEN** the command exits with a non-zero status
