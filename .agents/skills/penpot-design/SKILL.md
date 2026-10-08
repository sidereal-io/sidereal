---
name: penpot-design
description: Explore design options in Penpot within DESIGN.md's rules, so a person can compare them and pick one, then write the choice back to DESIGN.md. Use when a person asks to design, explore, or compare options for a component, screen, or token value in Penpot, or to sync Penpot's tokens with DESIGN.md.
---

# Penpot design

`DESIGN.md` is the source of truth for every token value and usage rule.
Penpot is where options are drawn and compared. A design change is real only
once it lands in `DESIGN.md`.

The work has three steps: **sync**, **explore**, and **decide**. Always sync
first, even if you only plan to look.

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
- the pages, including the page for explorations;
- the set that mirrors `DESIGN.md`'s primitive tokens, such as `primitives`;
- the naming pattern for exploration sets, such as `explore-<topic>`.

Then check that the connected file is the one `DESIGN.md` names, with a
read-only call to `penpot.currentFile`. If it is another file, stop and tell
the person.

## 1. Sync

1. **Get the token list from `main`.** The mirror set holds what has merged,
   so sync from an up-to-date `main`. If the checkout is on another branch,
   tell the person and ask before you go on. In the repository's dev shell,
   run `pnpm tokens:penpot` in `web/`. It prints a JSON array of
   `{ type, name, value }` built from `DESIGN.md`. If it fails, `DESIGN.md`
   holds a token it cannot write: report the error and stop.
2. **Run the sync.** Read `sync.js`, in this skill's folder. Call the Penpot
   `execute_code` tool with three lines followed by the contents of `sync.js`:

   ```js
   const setName = "<the mirror set DESIGN.md names>";
   const explorePrefix = "<the exploration prefix DESIGN.md names>";
   const tokens = <the JSON from step 1>;
   ```

3. **Report what changed.** `sync.js` returns the tokens it created, updated,
   retyped, and deleted. Tell the person the counts, and name the tokens.
4. **Confirm that the sync is complete.** Run it a second time. It must report
   nothing created, updated, retyped, or deleted. If it reports a change, stop
   and tell the person: Penpot is not holding a value the way the list states
   it.
5. **Ask about old explorations.** `sync.js` also returns every exploration
   set. If there are any, list them and ask which to delete. An exploration's
   set can go once its decision has merged into `DESIGN.md`, or once the person
   drops it. Delete only the sets the person names, each with
   `catalog.sets.find((s) => s.name === "<name>").remove()`, where `catalog` is
   `penpot.library.local.tokens`. Remove the set's boards from the
   explorations page too, if the person agrees.

**Never edit the mirror set by hand.** Only `sync.js` changes it.

## 2. Explore

1. **Agree on the topic.** Name what is being compared, in a word or two, such
   as `mono-tracking`. The topic names the exploration set and the boards.
2. **Put candidate values in their own set.** A value that is not yet in
   `DESIGN.md` goes in a new, active set named with the exploration prefix and
   the topic, such as `explore-mono-tracking`. Never add it to the mirror set.
   Name each candidate token for what it is, such as
   `letter-spacing-mono-12-a`.
3. **Draw two or three options.** Put each option on its own board on the
   explorations page, named `<topic> / <option letter>: <what it tries>`.
   Place the boards side by side.
   - Style every shape only with tokens, applied with `applyToken`. Do not type
     raw values.
   - Follow `DESIGN.md`'s rules: its usage tables, its contrast floor, and its
     rules for each component.
   - Show real content: real names, numbers, and lengths, not placeholder
     text.
4. **Show the options.** Export each board as an image, and tell the person
   where the boards are in Penpot. For each option, say in one line what it
   tries and what it costs.
5. **Ask the person to pick.** Do not choose for them. If they want changes,
   adjust the options and show them again.

**Penpot cannot hold every kind of value.**

- Penpot's letter-spacing tokens hold pixels only, and `DESIGN.md`'s letter
  spacing is in em. To try a letter spacing, work out pixels for each font
  size: em × font size. For example, 0.02em at 12px is 0.24px. Write the
  chosen value back in em.
- Penpot renders one font family. The sync keeps only the first family in
  each list.
- Penpot has no line-height token. Set a text's `lineHeight` to the ratio of
  `DESIGN.md`'s line height to its font size, such as `"1.5"` for 18px on
  12px. This is the one raw value allowed.

**Penpot's API has quirks.**

- Find a font by its exact name in `penpot.fonts.all`. `findByName` matches
  loosely: asked for `Inter`, it returns Inter Tight.
- Apply a font-family token without naming the property:
  `text.applyToken(token)`. Penpot rejects `["fontFamilies"]`.
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
4. **Leave the exploration set in place** until the change merges. The next
   sync from `main` copies the new values into the mirror set, and asks the
   person whether to delete the exploration set.

Nothing is written to `DESIGN.md` until the person picks an option.
