---
name: visual-options
description: Draw design options in Penpot within DESIGN.md's rules, so a person can compare them and pick one, then write the choice back to DESIGN.md. Use when a person asks to design, explore, or compare options for a component, screen, or token value in Penpot, or to sync Penpot's tokens with DESIGN.md and redraw its Tokens page.
---

# Visual options

`DESIGN.md` is the source of truth for every token value and usage rule.
Penpot is where options are drawn and compared. A design change is real only
once it lands in `DESIGN.md`.

The work has three steps: **sync**, **draw options**, and **decide**. Always
sync first, even if you only plan to look.

## Before you start

**You need the Penpot MCP tools,** with the Penpot plugin connected in the
design file.

- **The tools are not available:** tell the person that Penpot MCP is not
  connected for this agent, and stop.
- **A call fails because the plugin tab is asleep or not connected:** ask the
  person to focus the Penpot tab, or to connect the plugin again in the design
  file. Retry once. If it still fails, stop.

**Read the facts from `DESIGN.md`, not from this skill.** Its guidance on
Penpot names:

- the design file, and its ID;
- the Tokens page, such as `Tokens`;
- the token set that holds `DESIGN.md`'s primitive tokens, such as
  `primitives`;
- the naming pattern for an options page and its token set, such as
  `Options: <topic>` and `options/<topic>`.

Then check that the connected file is the one `DESIGN.md` names, with a
read-only call to `penpot.currentFile`. If it is another file, stop and tell
the person.

## 1. Sync

1. **Get the token list from `main`.** The `primitives` set holds what has
   merged, so sync from an up-to-date `main`. If the checkout is on another
   branch, tell the person and ask before you go on. In the repository's dev
   shell, run `pnpm tokens:penpot` in `web/`. It prints a JSON array of
   `{ group, type, name, value }` built from `DESIGN.md`. If it fails,
   `DESIGN.md` holds a token it cannot write: report the error and stop.
2. **Run the sync.** Read `scripts/sync.js`, in this skill's folder. Call the
   Penpot `execute_code` tool with three lines followed by the contents of
   `sync.js`:

   ```js
   const setName = "<the token set DESIGN.md names>";
   const optionsPrefix = "<the options set prefix DESIGN.md names>";
   const tokens = <the JSON from step 1>;
   ```

   To avoid pasting the list twice, store it first with
   `storage.tokens = <the JSON>`, then use `const tokens = storage.tokens;`
   here and in step 5.

3. **Report what changed.** `sync.js` returns the tokens it created, updated,
   retyped, and deleted. Tell the person the counts, and name the tokens.
4. **Confirm that the sync is complete.** Run it a second time. It must report
   nothing created, updated, retyped, or deleted. If it reports a change, stop
   and tell the person: Penpot is not holding a value the way the list states
   it.
5. **Redraw the Tokens page.** Read `scripts/draw-tokens.js`. Call
   `execute_code` with three lines followed by its contents:

   ```js
   const pageName = "<the Tokens page DESIGN.md names>";
   const setName = "<the token set DESIGN.md names>";
   const tokens = <the JSON from step 1>;
   ```

   It deletes everything on the page and draws every token again, each with
   a sample shape bound to the token. Run it after every sync, even when
   nothing changed. If it reports a group with no sample, add a sample for
   that group to `draw-tokens.js` in the same pull request as the new group.
6. **Ask about old options.** `sync.js` also returns every options set. If
   there are any, list them and ask which to delete. An option's set and page
   can go once its decision has merged into `DESIGN.md`, or once the person
   drops it. Delete only the ones the person names:
   - the set, with
     `catalog.sets.find((s) => s.name === "<name>").remove()`, where `catalog`
     is `penpot.library.local.tokens`;
   - its page, `Options: <topic>`, with `page.remove()`. Open another page
     first: Penpot cannot remove the page that is open.

**Never edit the `primitives` set or the Tokens page by hand.** Only
`sync.js` and `draw-tokens.js` change them.

## 2. Draw options

1. **Agree on the topic.** Name what is being compared, in a word or two,
   such as `mono-tracking`. The topic names the options page and its set.
2. **Make a page for the topic.** Create a page named with the options
   pattern, such as `Options: mono-tracking`, with `penpot.createPage()`, and
   open it with `await penpot.openPage(page)`. One topic, one page.
3. **Show what exists today.** If the topic is a component or screen that is
   already built, run the app with `just dev` and take a screenshot of it.
   Put the screenshot first on the page, on a board named `current`. Import
   it with `penpot.uploadMediaData`, and use it as an image fill. Options are
   compared against what is built, not against a redrawing.
4. **Put candidate values in their own set.** A value that is not yet in
   `DESIGN.md` goes in a new, active set named with the options pattern, such
   as `options/mono-tracking`. Never add it to the `primitives` set. Name each
   candidate token for what it is, such as `letter-spacing-mono-12-a`.
5. **Draw two or three options.** Put each option on its own board on the
   topic's page, named `<option letter>: <what it tries>`, such as
   `A: no tracking`. Place the boards side by side.
   - Style every shape only with tokens, applied with `applyToken`. Do not
     type raw values.
   - Follow `DESIGN.md`'s rules: its usage tables, its contrast floor, and its
     rules for each component.
   - Show real content: real names, numbers, and lengths, not placeholder
     text.
6. **Show the options.** Export each board as an image, and tell the person
   which page the boards are on. For each option, say in one line what it
   tries and what it costs.
7. **Ask the person to pick.** Do not choose for them. If they want changes,
   adjust the options and show them again.

**Penpot cannot hold every kind of value.**

- Penpot's letter-spacing tokens hold pixels only, and `DESIGN.md`'s letter
  spacing is in em. To try a letter spacing, work out pixels for each font
  size: em × font size. For example, 0.02em at 12px is 0.24px. Write the
  chosen value back in em.
- Penpot renders one font family. The sync keeps only the first family in
  each list.
- Penpot cannot bind a token to a text's line height. Set a text's
  `lineHeight` to the ratio of `DESIGN.md`'s line height to its font size,
  such as `"1.5"` for 18px on 12px. This is one of two raw values allowed.
- Penpot cannot bind a token to blur. Set `blur` to `DESIGN.md`'s value. This
  is the other raw value allowed.

**Penpot's API has quirks.**

- Shapes are created on the page that is open. Open a page before you draw
  on it.
- Find a font by its exact name in `penpot.fonts.all`. `findByName` matches
  loosely: asked for `Inter`, it returns Inter Tight.
- Apply a font-family token without naming the property:
  `text.applyToken(token)`. Penpot rejects `["fontFamilies"]`.
- A new rectangle has a grey fill. Set `fills = []` for a shape that should
  only have a stroke.
- A flex board settles its size after the call that builds it. Place boards
  side by side in a later call, once their widths are final.
- Export boards one at a time. Exporting the whole page can time out.

## 3. Decide

1. **Write the choice into `DESIGN.md`,** on a branch, not on `main`. Change
   the token values in the front matter. If the choice changes a rule, change
   the prose too. Keep `DESIGN.md`'s units: a pixel value tried in Penpot may
   need converting back, as em letter spacing does.
2. **Regenerate the CSS tokens.** Run `pnpm tokens` in `web/`.
3. **Check.** Run `just check` from the repository root. It must pass.
4. **Leave the options page and set in place** until the change merges. The
   next sync from `main` copies the new values into the `primitives` set,
   redraws the Tokens page, and asks the person whether to delete the options
   page and set.

Nothing is written to `DESIGN.md` until the person picks an option.
