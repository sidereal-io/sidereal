// Makes a Penpot token set match DESIGN.md, then lists the options sets.
// Run it through the Penpot MCP execute_code tool. Put three lines before it:
//
//   const setName = "primitives";        // the token set DESIGN.md names
//   const optionsPrefix = "options/";    // the options set prefix DESIGN.md names
//   const tokens = [ ... ];              // the output of `pnpm tokens:penpot`
//
// It creates missing tokens, updates changed values, deletes tokens that are
// not in the list, and deletes and recreates a token whose type changed.
// Running it twice in a row changes nothing the second time.

const catalog = penpot.library.local.tokens;
let set = catalog.sets.find((s) => s.name === setName);
if (!set) set = catalog.addSet({ name: setName, active: true });
if (!set.active) set.toggleActive();

// Penpot may hand back a font family list as an array.
const valueOf = (token) =>
  Array.isArray(token.value) ? token.value.join(", ") : String(token.value);

const wanted = new Map(tokens.map((token) => [token.name, token]));
const report = { created: [], updated: [], retyped: [], deleted: [] };

for (const token of [...set.tokens]) {
  // Read the name first: a removed token no longer has one.
  const name = token.name;
  const want = wanted.get(name);
  if (want && want.type === token.type) continue;
  token.remove();
  (want ? report.retyped : report.deleted).push(name);
}

for (const want of tokens) {
  const token = set.tokens.find((t) => t.name === want.name);
  if (!token) {
    set.addToken({ type: want.type, name: want.name, value: want.value });
    if (!report.retyped.includes(want.name)) report.created.push(want.name);
  } else if (valueOf(token) !== want.value) {
    token.value = want.value;
    report.updated.push(want.name);
  }
}

return {
  set: setName,
  created: report.created,
  updated: report.updated,
  retyped: report.retyped,
  deleted: report.deleted,
  optionsSets: catalog.sets
    .filter((s) => s.name.startsWith(optionsPrefix))
    .map((s) => ({ name: s.name, tokens: s.tokens.length })),
};
