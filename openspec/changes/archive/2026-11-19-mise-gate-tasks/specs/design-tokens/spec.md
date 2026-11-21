## MODIFIED Requirements

### Requirement: Stylesheets use only tokens that exist

The unit tests SHALL fail when a stylesheet or TypeScript file under `web/src/` uses, through `var()`, a custom property that no stylesheet under `web/src/` defines. `tokens.css` counts as one of those stylesheets. The test SHALL check every `var()` name, whatever its prefix. The test's output SHALL name the property and the file that uses it.

#### Scenario: The prefix changed, but the stylesheets did not

- **WHEN** a tester changes `x-sidereal.css-prefix` in `DESIGN.md` to `foo`, runs the generator, and runs `mise run check-web`
- **THEN** the command exits with a non-zero status
- **AND** its output contains `--sr-obsidian-950`

#### Scenario: A token in use was removed

- **WHEN** a tester removes `obsidian-100` from `DESIGN.md`, runs the generator, and runs `mise run check-web`
- **THEN** the command exits with a non-zero status
- **AND** its output contains `--sr-obsidian-100`

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
