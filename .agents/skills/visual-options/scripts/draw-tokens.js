// Draws every token in a Penpot token set on one page, each with a sample
// shape bound to the token. Run it through the Penpot MCP execute_code tool,
// after sync.js. Put three lines before it:
//
//   const pageName = "Tokens";           // the page DESIGN.md names
//   const setName = "primitives";        // the token set DESIGN.md names
//   const tokens = [ ... ];              // the output of `pnpm tokens:penpot`
//
// It owns the whole page: it deletes everything on the page and draws it
// again, so running it twice gives the same page. The page itself is styled
// only with tokens from the same set.

const catalog = penpot.library.local.tokens;
const set = catalog.sets.find((s) => s.name === setName);
if (!set)
  throw new Error(`There is no token set named ${setName}. Run sync.js first.`);

const token = (name) => {
  const found = set.tokens.find((t) => t.name === name);
  if (!found) {
    throw new Error(
      `draw-tokens.js needs the token ${name}, and ${setName} has none. Run sync.js first, or change the token names in draw-tokens.js.`,
    );
  }
  return found;
};

// The tokens that style the page itself.
const page = {
  background: "obsidian-950",
  text: "obsidian-100",
  quiet: "obsidian-500",
  outline: "obsidian-800",
  accent: "violet-500",
  sans: "font-family-sans",
  mono: "font-family-mono",
  regular: "font-weight-400",
  strong: "font-weight-600",
  borderWidth: "border-width-1",
  radius: "radius-4",
};

const sampleText = "Ag 0O 1lI 5S";
const corners = [
  "borderRadiusTopLeft",
  "borderRadiusTopRight",
  "borderRadiusBottomRight",
  "borderRadiusBottomLeft",
];

// --- Building blocks -------------------------------------------------------

function text(characters, { family, size, weight, color }) {
  const shape = penpot.createText(characters);
  shape.name = characters;
  shape.growType = "auto-width";
  // Penpot rejects ["fontFamilies"]; apply a family without a property.
  shape.applyToken(token(family));
  shape.applyToken(token(size), ["fontSize"]);
  shape.applyToken(token(weight), ["fontWeight"]);
  shape.applyToken(token(color), ["fill"]);
  return shape;
}

function stack(name, dir, gap) {
  const board = penpot.createBoard();
  board.name = name;
  board.fills = [];
  board.clipContent = false;
  const flex = board.addFlexLayout();
  flex.dir = dir;
  flex.horizontalSizing = "auto";
  flex.verticalSizing = "auto";
  if (gap)
    board.applyToken(token(gap), dir === "row" ? ["columnGap"] : ["rowGap"]);
  return board;
}

function rectangle(name, width, height) {
  const shape = penpot.createRectangle();
  shape.name = name;
  shape.resize(width, height);
  return shape;
}

function outline(shape) {
  shape.strokes = [
    {
      strokeColor: "#000000",
      strokeOpacity: 1,
      strokeWidth: 1,
      strokeAlignment: "inner",
      strokeStyle: "solid",
    },
  ];
  shape.applyToken(token(page.outline), ["strokeColor"]);
  shape.applyToken(token(page.borderWidth), ["strokeWidth"]);
}

function fill(shape, name) {
  shape.fills = [{ fillColor: "#000000", fillOpacity: 1 }];
  shape.applyToken(token(name), ["fill"]);
}

// --- One sample per group --------------------------------------------------

// Each function draws the sample shape for one token, bound to that token.
const samples = {
  colors(t) {
    const shape = rectangle("sample", 96, 48);
    fill(shape, t.name);
    outline(shape);
    shape.applyToken(token(page.radius), corners);
    return shape;
  },
  rounded(t) {
    const shape = rectangle("sample", 48, 48);
    fill(shape, page.outline);
    shape.applyToken(token(t.name), corners);
    return shape;
  },
  // Two blocks with the spacing between them.
  spacing(t) {
    const board = stack("sample", "row", null);
    board.applyToken(token(t.name), ["columnGap"]);
    for (const side of ["start", "end"]) {
      const block = rectangle(side, 8, 24);
      fill(block, page.accent);
      board.appendChild(block);
    }
    return board;
  },
  "font-family"(t) {
    return text(sampleText, {
      family: t.name,
      size: "font-size-20",
      weight: page.regular,
      color: page.text,
    });
  },
  "font-weight"(t) {
    return text(sampleText, {
      family: page.sans,
      size: "font-size-20",
      weight: t.name,
      color: page.text,
    });
  },
  "font-size"(t) {
    return text(sampleText, {
      family: page.sans,
      size: t.name,
      weight: page.regular,
      color: page.text,
    });
  },
  // A box as tall as the line, around one line of text.
  "line-height"(t) {
    const board = stack("sample", "row", null);
    board.flex.verticalSizing = "fix";
    board.flex.alignItems = "center";
    board.applyToken(token("spacing-8"), ["paddingLeft", "paddingRight"]);
    board.applyToken(token(t.name), ["height"]);
    fill(board, page.outline);
    board.appendChild(
      text(sampleText, {
        family: page.sans,
        size: "font-size-12",
        weight: page.regular,
        color: page.text,
      }),
    );
    return board;
  },
  opacity(t) {
    const shape = rectangle("sample", 96, 48);
    fill(shape, page.text);
    shape.applyToken(token(t.name), ["opacity"]);
    return shape;
  },
  // Penpot cannot bind a token to blur, so this sample uses the raw value.
  blur(t) {
    const shape = rectangle("sample", 48, 48);
    fill(shape, page.accent);
    shape.blur = { value: Number(t.value) };
    return shape;
  },
  "border-width"(t) {
    const shape = rectangle("sample", 48, 48);
    shape.fills = [];
    shape.strokes = [
      {
        strokeColor: "#000000",
        strokeOpacity: 1,
        strokeWidth: 1,
        strokeAlignment: "inner",
        strokeStyle: "solid",
      },
    ];
    shape.applyToken(token(page.text), ["strokeColor"]);
    shape.applyToken(token(t.name), ["strokeWidth"]);
    return shape;
  },
  // A bar as long as the size. A large size, such as 1280, draws full length.
  size(t) {
    const shape = rectangle("sample", 8, 8);
    fill(shape, page.accent);
    shape.applyToken(token(t.name), ["width"]);
    return shape;
  },
};

// Notes shown under a group's title, where a sample needs explaining.
const notes = {
  "line-height":
    "Each box is bound to its token for height. Penpot cannot bind a token to a text's line height: set it as a ratio, line height ÷ font size.",
  blur: "Penpot cannot bind a token to blur, so these samples use the raw value.",
};

// Values in pixels, as DESIGN.md writes them.
const pixelGroups = [
  "rounded",
  "spacing",
  "font-size",
  "line-height",
  "blur",
  "border-width",
  "size",
];

const title = (group) => {
  const words = group.replace(/-/g, " ");
  return words[0].toUpperCase() + words.slice(1);
};

// --- The page --------------------------------------------------------------

let target = penpotUtils.getPageByName(pageName);
if (!target) {
  target = penpot.createPage();
  target.name = pageName;
}
await penpot.openPage(target);
for (const shape of [...target.root.children]) shape.remove();

const groups = [];
for (const t of tokens) {
  if (!samples[t.group]) {
    throw new Error(
      `draw-tokens.js has no sample for the ${t.group} group. Add one to samples.`,
    );
  }
  let group = groups.find((g) => g.name === t.group);
  if (!group) groups.push((group = { name: t.group, tokens: [] }));
  group.tokens.push(t);
}

const root = stack(pageName, "column", "spacing-32");
root.flex.horizontalSizing = "fix";
root.resize(1400, 100);
root.flex.verticalSizing = "auto";
root.applyToken(token("spacing-32"), [
  "paddingLeft",
  "paddingRight",
  "paddingTop",
  "paddingBottom",
]);
fill(root, page.background);
root.x = 0;
root.y = 0;

const intro = text(
  `Every token in the ${setName} set, drawn from DESIGN.md. Each sample is bound to its token. draw-tokens.js redraws this page after every sync, so do not edit it by hand. Letter spacing is not drawn: Penpot's letter-spacing tokens hold pixels, and DESIGN.md's are in em.`,
  {
    family: page.sans,
    size: "font-size-14",
    weight: page.regular,
    color: page.quiet,
  },
);
intro.name = "About this page";
root.appendChild(
  text(pageName, {
    family: page.sans,
    size: "font-size-30",
    weight: page.strong,
    color: page.text,
  }),
);
root.appendChild(intro);
intro.growType = "auto-height";
intro.resize(960, intro.height);
intro.growType = "auto-height";

for (const group of groups) {
  const board = stack(title(group.name), "column", "spacing-12");
  root.appendChild(board);
  board.layoutChild.horizontalSizing = "fill";
  board.appendChild(
    text(title(group.name), {
      family: page.sans,
      size: "font-size-20",
      weight: page.strong,
      color: page.text,
    }),
  );
  if (notes[group.name]) {
    board.appendChild(
      text(notes[group.name], {
        family: page.sans,
        size: "font-size-12",
        weight: page.regular,
        color: page.quiet,
      }),
    );
  }

  const entries = stack("entries", "row", "spacing-24");
  board.appendChild(entries);
  entries.flex.wrap = "wrap";
  entries.applyToken(token("spacing-20"), ["rowGap"]);
  entries.layoutChild.horizontalSizing = "fill";

  for (const t of group.tokens) {
    const entry = stack(t.name, "column", "spacing-4");
    entries.appendChild(entry);
    entry.appendChild(samples[group.name](t));
    entry.appendChild(
      text(t.name, {
        family: page.mono,
        size: "font-size-12",
        weight: page.regular,
        color: page.text,
      }),
    );
    const value = pixelGroups.includes(group.name) ? `${t.value}px` : t.value;
    entry.appendChild(
      text(value, {
        family: page.mono,
        size: "font-size-11",
        weight: page.regular,
        color: page.quiet,
      }),
    );
  }
}

return {
  page: pageName,
  drawn: tokens.length,
  groups: groups.map((g) => ({ group: g.name, tokens: g.tokens.length })),
};
