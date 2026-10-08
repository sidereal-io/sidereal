## Why

`DESIGN.md` defines Sidereal's design tokens, but the web shell does not use them yet. The first component stories in #337 need those tokens in code. Without them, each component would hard-code colors and sizes that someone later has to find and remove. Contributors also need `just check` to catch the moment code, `DESIGN.md`, or the contrast table drift apart.

The issue also changes how tokens reach Penpot. An agent now copies tokens into Penpot over the Penpot MCP server, so the repo no longer needs a token JSON file or a manual import. ADR-005 still names that file and that import, so this change amends it.

## What Changes

- A token generator in `web/` reads the front matter of `DESIGN.md`, including the `x-sidereal` extension. It writes `web/src/styles/tokens.css`, with one `--sr-{name}` custom property per primitive token. Its check mode compares the file with `DESIGN.md` and writes nothing.
- `just check-web` gains three checks:
  - the `DESIGN.md` linter, which fails on errors and allows warnings;
  - a drift check, which fails when `tokens.css` does not match `DESIGN.md`;
  - a contrast test, which fails when any ratio in `DESIGN.md`'s contrast table is wrong.
- The web shell serves Inter and Atkinson Hyperlegible Mono itself, from Fontsource packages. It loads no font from another site.
- The health screen uses `obsidian-950` for its background, `obsidian-100` for its text, and the sans font. Each value comes through a token.
- ADR-005 is amended in place: an agent keeps Penpot's tokens current, and `tokens.css` is the only generated file. `DESIGN.md`, `AGENTS.md`, and `openspec/config.yaml` drop the JSON file and the manual import to match.

## Capabilities

### New Capabilities

- `design-tokens`: the web shell's tokens are generated from `DESIGN.md`, and the web gate catches drift between them, linter errors in `DESIGN.md`, and wrong contrast claims.

### Modified Capabilities

- `dev-commands`: the web gate runs seven checks instead of four. The new ones are the `DESIGN.md` linter, the token drift check, and the contrast test.
- `web-shell`: the web shell serves its own fonts and styles the health screen through the tokens.

## Impact

- **New files:** the generator and its shared `DESIGN.md` reader under `web/scripts/`, the contrast test, `web/src/styles/tokens.css`, and a base stylesheet with the font rules.
- **Changed code:** `web/src/main.tsx` (imports the stylesheets), `web/package.json` and `web/pnpm-lock.yaml`, `web/tsconfig.node.json`, `web/.prettierignore`, and `justfile` (`check-web`).
- **Dependencies:** two runtime packages, the Fontsource variable packages for the two fonts. Two dev dependencies: the `@google/design.md` linter and a YAML parser. The Nix shell needs no new tool.
- **Docs:** ADR-005, `DESIGN.md`, `AGENTS.md`, `openspec/config.yaml`, and `web/README.md`.
- **Contributors:** a token change now needs one more command, the generator, before `just check` passes.
- **Out of scope:** an adapter such as Tailwind; components; semantic and component tokens, and the reference lookup they will need; light mode; any Penpot sync, and the mono letter-spacing check (#381).
