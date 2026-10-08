## Context

See proposal.md for why. This section covers only the facts that shape the approach.

- **`DESIGN.md` keeps its values in two places.** The DESIGN.md specification's own keys (`colors`, `rounded`, `spacing`) hold some values. Sidereal's `x-sidereal` extension holds the rest: type values, opacity, blur, border width, and sizes. Today there are 60 primitives and no semantic or component tokens.
- **The specification's tool, `@google/design.md` 0.4.0, cannot generate the file.** Its `css-vars` export writes `--color-obsidian-950`, not `--sr-obsidian-950`, and it ignores `x-sidereal`.
- **Its linter already passes.** On today's `DESIGN.md`, it reports no errors and three warnings, and exits with status 0. With an invalid color, it exits with status 1.
- **Fontsource names its fonts differently from `DESIGN.md`.** The variable packages register `Inter Variable` and `Atkinson Hyperlegible Mono Variable`. `DESIGN.md`'s font stacks start with `Inter` and `Atkinson Hyperlegible Mono`. With Fontsource's standard imports, the browser would never use the self-hosted files.
- **The repo pins Node 26.** Node 26 runs TypeScript files directly, with no build step.
- **The web gate does not build the app.** `just check-web` runs type checks, lint, format, and unit tests, but not `vite build`. A broken file path in a stylesheet would not fail the gate on its own.

## Goals / Non-Goals

**Goals:**

- A token change takes two steps: edit `DESIGN.md`, then run one command.
- Each check fails with a message that says what to do next.
- The generator stays small enough to read in one sitting.

**Non-Goals:**

- A general token pipeline, such as Style Dictionary, with platforms and transforms.
- Resolving references such as `{colors.violet-600}`. The first semantic token brings that.
- Checking the ratio sentences under the contrast table. Only the table is checked.

## Decisions

### D1. A small generator in `web/scripts/`, run by Node

`web/scripts/tokens.ts` writes `web/src/styles/tokens.css`. Node runs the file directly. `web/package.json` gains two scripts: `tokens` writes the file, and `tokens:check` compares the file with `DESIGN.md` and writes nothing.

- **Why `web/`:** the script needs a YAML parser, and only `web/` has a package manager. The repo root has no npm project, by design.
- **Alternative: the specification's exporter.** Rejected, because it uses other names and drops `x-sidereal` (see Context).
- **Alternative: Style Dictionary.** Rejected. It needs its own config and a transform for `x-sidereal`. That is more code than the generator itself for 60 tokens.

### D2. One shared reader for `DESIGN.md`

`web/scripts/design-md.ts` reads `DESIGN.md`, splits off the front matter between the first two `---` lines, and parses it with the `yaml` package. The generator and the contrast test both use it. So both always read `DESIGN.md` the same way.

### D3. The output is plain, ordered, and owned by the generator

`tokens.css` starts with a comment: the file is generated from `DESIGN.md`, by `pnpm tokens`, and must not be edited. Then one `:root` block lists the properties in `DESIGN.md`'s order, with a comment above each group. Values are copied exactly as `DESIGN.md` writes them. The prefix comes from `x-sidereal.css-prefix`.

- **Prettier skips the file.** `web/.prettierignore` lists it. The generator alone decides its bytes, so the drift check and the format check never disagree.
- **The type check covers the scripts.** `web/tsconfig.node.json` adds `scripts/**/*.ts` to its `include` list.

### D4. The generator refuses what it cannot write safely

The generator exits with an error, naming the token, in these cases:

- `x-sidereal.semantic` or `components` holds an entry;
- two groups share a key;
- a key holds anything other than lowercase letters, digits, and hyphens;
- `x-sidereal.css-prefix` holds anything other than lowercase letters;
- a value holds a line break, `;`, `{`, `}`, `<`, `\`, or `url(`;
- a value does not fit its group's form. For example, `font-size` takes whole pixels, and `font-family` takes a font list;
- an `x-sidereal` group has no known form.

- **Why not skip semantic and component tokens?** A skipped token would not reach `tokens.css`. A component could then use a CSS property that does not exist, and no check would notice. The error tells the first semantic token's author to add reference lookup in the same pull request.
- **Why check keys, values, and the prefix?** The generator copies each of them into CSS exactly. Without the check, a value could close its property and add a rule, such as one that loads a file from another site. The linter catches this under `colors`, but not under `x-sidereal`. The author tested both. No current token uses any of the refused characters. The font stacks use quotes and commas, which stay allowed.
- **Why check each group's form?** The linter ignores `x-sidereal`, so nothing else checks those values. A `font-family-sans` of `12px` would pass the drift check, but the browser would drop the font. The forms live in the generator, one per group. A new group fails until its author adds a form, which keeps the list complete.

### D5. The `DESIGN.md` linter is a pinned dev dependency

`web/package.json` lists `@google/design.md` at an exact version. Its `design:lint` script runs `web/scripts/design-lint.ts`. That script runs the linter on `../DESIGN.md` and reads the linter's JSON report.

- **Why not `npx`?** `npx @google/design.md` fetches the newest release on each run. The specification is in alpha, so a new release could break the gate with no change in the repo. A pinned version changes only through a Dependabot pull request, where any new finding shows up.
- **Why read the report, not just the exit status?** The linter exits with status 0 on any number of warnings. The author added an unknown top-level key, and the linter reported a fourth warning and still exited with 0. The script fails on any error, and on any warning that is not in its list of three known warnings. It matches each by rule and path, so a second ignored key, for example, still fails. The script prints each unexpected finding.
- **A known warning may disappear.** The first typography token, for example, removes `missing-typography`. The gate still passes, and the author can drop that entry from the list.

### D6. The contrast test reads the table, not a copy of it

`web/scripts/contrast.test.ts` runs with the unit tests, in Vitest's Node environment. It finds the `### Contrast` section and parses the table's header and rows. It computes each ratio with the WCAG 2 relative-luminance formula, from the front matter's hex values.

- **Rounding:** the test rounds the ratio half up to one decimal before it compares. The pass or fail mark uses the unrounded ratio. So `obsidian-500` on `obsidian-850` is 4.507: it shows as `4.5` and passes.
- **Threshold:** every pair in the table is body text, so every cell uses 4.5:1.
- **A broken table fails loudly.** The test fails when it finds no table, no background columns, no rows, or a name that is not a color token. It also fails on any cell that is not a one-decimal ratio, that ratio followed by `(fails)`, or `—`. A reformatted table can never pass by checking nothing.

### D7. The web shell's own `@font-face` rules, using Fontsource's files

`web/src/styles/fonts.css` declares two `@font-face` rules, with the family names `Inter` and `Atkinson Hyperlegible Mono`. Each rule points at its Fontsource package's Latin variable file, `*-latin-wght-normal.woff2`. Each declares `font-weight: 400 600`, `font-display: swap`, and Fontsource's Latin `unicode-range`. Vite bundles the files, so the web shell serves them from its own origin.

- **Why not Fontsource's standard import?** It registers `Inter Variable`, which no font stack in `DESIGN.md` names (see Context).
- **Alternative: put `Inter Variable` first in `DESIGN.md`'s font stacks.** Rejected. A package's naming would leak into the source of truth, and into Penpot, which knows the font only as `Inter`.
- **A unit test guards the file paths.** `web/src/styles/fonts.test.ts` checks that each `url()` in `fonts.css` names a file that exists. A Fontsource update that renames a file then fails the gate, not just the running app.
- **Fontsource is a runtime dependency.** The font files ship with the app, so both packages go under `dependencies`.

### D8. A base stylesheet applies the tokens to the health screen

`web/src/styles/base.css` styles `body` with `var(--sr-obsidian-950)`, `var(--sr-obsidian-100)`, and `var(--sr-font-family-sans)`, and removes the browser's default margin. `web/src/main.tsx` imports `tokens.css`, `fonts.css`, and `base.css`, in that order. The health screen's components do not change.

A unit test, `web/src/styles/tokens-in-use.test.ts`, reads every stylesheet under `web/src/` except `tokens.css`. It fails when a stylesheet uses, through `var()`, a custom property that no stylesheet defines. It checks every name, whatever its prefix. So after a prefix rename, the old `--sr-…` uses in `base.css` fail. It also catches a removed token that code still uses, and a typo in a `var()`.

**New semantic or component tokens:** none. The body styles use primitives directly, which `DESIGN.md` allows until a semantic token for that role exists.

### D9. ADR-005 is amended in place, with the docs that repeat it

The maintainer chose to amend ADR-005, not to replace it with a new ADR. No code ever used the token JSON file, so one changed point does not justify a new record. One commit updates every place that names the JSON file or the manual import:

- **ADR-005:** an agent copies the tokens into Penpot over the Penpot MCP server before it works there. Penpot may lag behind `DESIGN.md` until then. `tokens.css` is the only generated file.
- **`DESIGN.md`:** "How to use this file" and "Changing any token" say to run `pnpm tokens`, and that Penpot gets its tokens from an agent.
- **`AGENTS.md`:** the third web UI rule changes the same way.
- **`openspec/config.yaml`:** the `tasks` rule asks for a task to regenerate `tokens.css`, not a Penpot import.

### D10. CI runs when `DESIGN.md` changes

`.github/workflows/ci.yml` adds `DESIGN.md` to both of its path filters, for pull requests and for pushes to `main`. Today a pull request that changes only `DESIGN.md` triggers no CI. The drift, lint, and contrast checks would then never run on the most common token change. Both jobs share one path filter, so a `DESIGN.md` change also runs the `server` job. That costs a few minutes, and keeps the workflow's single filter, which the `ci` spec requires.

## Risks / Trade-offs

- **[The specification is alpha, and its linter may add rules.]** → The version is pinned (D5). A new error appears only in the Dependabot pull request that brings it.
- **[A contributor edits `DESIGN.md` and forgets the generator.]** → The drift check fails and prints the command to run, both locally and in CI (D10).
- **[A generated file is committed.]** → Two branches that both change tokens conflict in `tokens.css`. The fix is to regenerate after merging `DESIGN.md`.
- **[The contrast test depends on the table's layout.]** → A layout change fails the test instead of passing silently (D6). The author then updates the parser or the table.
- **[The fonts are declared from 400 to 600, but each file holds the full weight axis.]** → Download size stays the same. The range only stops the browser from using weights that `DESIGN.md` does not define.
- **[Penpot lags behind `DESIGN.md` after a token change merges.]** → This lag is intended. Penpot gets the new tokens at the start of the next exploration, when an agent copies them (#381). Between explorations, no work reads Penpot's tokens: code reads `tokens.css`, and `tokens.css` comes from `DESIGN.md`. No check in the repo can reach Penpot.

## Migration Plan

No migration is needed. The change adds files and checks. To roll it back, revert the pull request.
