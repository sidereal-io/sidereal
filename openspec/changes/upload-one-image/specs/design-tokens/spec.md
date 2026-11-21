## MODIFIED Requirements

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

- a semantic token whose value is not a `{<group>.<key>}` reference to a primitive;
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
