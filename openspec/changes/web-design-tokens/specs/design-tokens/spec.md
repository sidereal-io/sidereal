## Purpose

The web shell's design tokens come from `DESIGN.md`, the source of truth for how screens look. The web gate fails when the generated tokens, `DESIGN.md` itself, or its contrast claims drift.

## ADDED Requirements

### Requirement: The token file holds every primitive from DESIGN.md

`web/src/styles/tokens.css` SHALL define one CSS custom property for each primitive token in `DESIGN.md`'s front matter. The primitives are the entries under `colors`, `rounded`, and `spacing`, and under each group in `x-sidereal` except `css-prefix` and `semantic`.

Each property's name SHALL be `--`, then the value of `x-sidereal.css-prefix`, then `-`, then the token's key. Each property's value SHALL be the token's value as `DESIGN.md` writes it. The file SHALL define no other custom property.

The generator SHALL write this file. Contributors SHALL NOT edit it by hand.

#### Scenario: A color token

- **WHEN** a reviewer reads `web/src/styles/tokens.css`
- **THEN** it defines `--sr-obsidian-950` with the value `#0B0A0F`

#### Scenario: A token from the extension

- **WHEN** a reviewer reads `web/src/styles/tokens.css`
- **THEN** it defines `--sr-font-size-14` with the value `14px`
- **AND** it defines `--sr-opacity-45` with the value `0.45`

#### Scenario: One property per primitive

- **WHEN** a tester counts the primitive tokens in `DESIGN.md`'s front matter and the custom properties in `web/src/styles/tokens.css`
- **THEN** both counts are equal
- **AND** every primitive's key appears in exactly one property name

#### Scenario: A new token

- **WHEN** a tester adds `spacing-16: 16px` under `spacing` in `DESIGN.md`
- **AND** runs the generator
- **THEN** `web/src/styles/tokens.css` defines `--sr-spacing-16` with the value `16px`

### Requirement: The generator refuses tokens it cannot write yet

The generator SHALL exit with a non-zero status and write no file when `DESIGN.md` holds any token it cannot write. These are:

- any entry under `x-sidereal.semantic`;
- any entry under `components`;
- two tokens with the same key in different groups.

Its error message SHALL name the token.

#### Scenario: A semantic token

- **WHEN** a tester adds `color-action-primary: "{colors.violet-600}"` under `x-sidereal.semantic` in `DESIGN.md`
- **AND** runs the generator
- **THEN** the generator exits with a non-zero status
- **AND** its output contains `color-action-primary`
- **AND** `git diff --exit-code web/src/styles/tokens.css` exits with status 0

#### Scenario: A duplicate key

- **WHEN** a tester adds `spacing-4: 4px` under `x-sidereal.size` in `DESIGN.md`
- **AND** runs the generator
- **THEN** the generator exits with a non-zero status
- **AND** its output contains `spacing-4`

### Requirement: The web gate catches a stale token file

The token drift check SHALL compare `web/src/styles/tokens.css` with the output the generator would write from `DESIGN.md`. It SHALL exit with a non-zero status when they differ, and print the command that regenerates the file. It SHALL NOT change any file.

#### Scenario: DESIGN.md changed, but the token file did not

- **WHEN** a tester changes `obsidian-950` to `#0C0B10` in `DESIGN.md`, without running the generator
- **AND** runs `just check-web`
- **THEN** the command exits with a non-zero status
- **AND** its output names the command that regenerates the token file
- **AND** `git diff --exit-code web/src/styles/tokens.css` exits with status 0

#### Scenario: The token file was edited by hand

- **WHEN** a tester changes the value of `--sr-obsidian-950` in `web/src/styles/tokens.css`
- **AND** runs `just check-web`
- **THEN** the command exits with a non-zero status

#### Scenario: The token file was regenerated

- **WHEN** a tester changes `obsidian-950` in `DESIGN.md`, runs the generator, and runs `just check-web`
- **THEN** the token drift check passes

### Requirement: The web gate lints DESIGN.md

The web gate SHALL run the DESIGN.md specification's linter on `DESIGN.md`. A linter error SHALL fail the gate. A linter warning SHALL NOT fail the gate.

#### Scenario: DESIGN.md has only the known warnings

- **WHEN** a contributor runs `just check-web` on an unchanged checkout
- **THEN** the linter step passes

#### Scenario: A color value is not a color

- **WHEN** a tester changes `obsidian-950` to `"#GGGGGG"` in `DESIGN.md`
- **AND** runs `just check-web`
- **THEN** the command exits with a non-zero status

### Requirement: The web gate checks DESIGN.md's contrast claims

The unit tests SHALL check every cell of the table in `DESIGN.md`'s Contrast section. The first cell of each row names a text color token. Each column header names a background color token. For each cell that holds a number, the test SHALL compute the WCAG 2 contrast ratio between the two tokens' values in the front matter. The test SHALL fail when:

- the computed ratio, rounded to one decimal place, differs from the number in the cell;
- the cell is marked `(fails)`, but the computed ratio is at least 4.5;
- the cell is not marked `(fails)`, but the computed ratio is below 4.5;
- the table is missing, or names a token that is not a color in the front matter.

A cell that holds `—` claims nothing, and the test SHALL skip it.

#### Scenario: The table is correct

- **WHEN** a contributor runs `just check-web` on an unchanged checkout
- **THEN** the contrast test passes

#### Scenario: A color changed, but its claims did not

- **WHEN** a tester changes `obsidian-500` to `#6F6B7C` in `DESIGN.md`
- **AND** runs `just check-web`
- **THEN** the command exits with a non-zero status

#### Scenario: A claimed ratio is wrong

- **WHEN** a tester changes the `obsidian-400` cell on `obsidian-950` from `7.2` to `7.5`
- **AND** runs `just check-web`
- **THEN** the command exits with a non-zero status

#### Scenario: A failing pair is not marked

- **WHEN** a tester removes `(fails)` from the `obsidian-600` cell on `obsidian-950`
- **AND** runs `just check-web`
- **THEN** the command exits with a non-zero status
