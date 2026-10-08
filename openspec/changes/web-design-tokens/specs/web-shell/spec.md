## ADDED Requirements

### Requirement: The web shell serves its own fonts

The web shell SHALL serve the font files for Inter and Atkinson Hyperlegible Mono from its own origin. It SHALL register each font under the family name that `DESIGN.md`'s font tokens put first: `Inter` and `Atkinson Hyperlegible Mono`. It SHALL NOT load a font, or anything else, from another origin.

#### Scenario: Inter loads from the web shell

- **WHEN** a user opens the web shell's home page and the page finishes loading
- **THEN** `[...document.fonts].some(f => f.family === "Inter" && f.status === "loaded")` returns `true` in the browser console
- **AND** the browser's network log shows that the Inter font file came from the web shell's origin

#### Scenario: The mono font is registered

- **WHEN** a user opens the web shell's home page
- **THEN** `[...document.fonts].some(f => f.family === "Atkinson Hyperlegible Mono")` returns `true` in the browser console

#### Scenario: Nothing loads from another origin

- **WHEN** a user opens the web shell's home page and the page finishes loading
- **THEN** the browser's network log shows no request to any origin other than the web shell's

### Requirement: The health screen uses the design tokens

The health screen SHALL take its page background, its text color, and its font family from the token file, not from values written in its own styles. It SHALL use `obsidian-950` for the page background, `obsidian-100` for text, and `font-family-sans` for the font.

#### Scenario: The screen shows the token values

- **WHEN** a user opens the web shell's home page
- **THEN** `getComputedStyle(document.body).backgroundColor` returns `rgb(11, 10, 15)`
- **AND** `getComputedStyle(document.body).color` returns `rgb(245, 244, 239)`
- **AND** `getComputedStyle(document.body).fontFamily` starts with `Inter`

#### Scenario: A token change reaches the screen

- **WHEN** a tester changes `obsidian-950` to `#200000` in `DESIGN.md`, runs the generator, and reloads the home page
- **THEN** `getComputedStyle(document.body).backgroundColor` returns `rgb(32, 0, 0)`
