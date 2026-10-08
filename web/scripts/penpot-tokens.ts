// Prints DESIGN.md's primitive tokens as the list that the Penpot scripts
// read: a JSON array of { group, type, name, value }, in DESIGN.md's order.
//
//   pnpm tokens:penpot
import { fileURLToPath } from "node:url";
import { readDesignMd } from "./design-md.ts";
import { TokenError, checkTokens, primitiveGroups } from "./tokens.ts";

export interface PenpotToken {
  // The DESIGN.md group the token comes from, such as `colors` or
  // `line-height`. The Tokens page draws one board per group.
  group: string;
  // A Penpot token type, such as `color` or `fontSizes`.
  type: string;
  name: string;
  // Penpot stores every token value as a string.
  value: string;
}

export function main(): number {
  let tokens: PenpotToken[];
  try {
    tokens = buildPenpotTokens(readDesignMd().frontMatter);
  } catch (error) {
    if (!(error instanceof TokenError)) throw error;
    console.error(error.message);
    return 1;
  }
  console.log(JSON.stringify(tokens, null, 2));
  return 0;
}

// Returns one Penpot token per primitive, except the groups Penpot cannot
// hold. Throws a TokenError for the same tokens that tokens.css refuses, and
// for a group with no Penpot type.
export function buildPenpotTokens(
  frontMatter: Record<string, unknown>,
): PenpotToken[] {
  checkTokens(frontMatter);

  const tokens: PenpotToken[] = [];
  for (const group of primitiveGroups(frontMatter)) {
    if (skippedGroups.includes(group.path)) continue;
    const type = penpotTypes[group.path];
    if (type === undefined) {
      throw new TokenError(
        `${group.path} has no Penpot type. Add one to penpotTypes in web/scripts/penpot-tokens.ts, or add the group to skippedGroups.`,
      );
    }
    const groupName = group.path.split(".").at(-1) ?? group.path;
    for (const [name, value] of Object.entries(group.tokens)) {
      tokens.push({
        group: groupName,
        type,
        name,
        value: penpotValue(group.path, value),
      });
    }
  }
  return tokens;
}

// The Penpot token type for each primitive group. Penpot has no line-height
// or blur type, so those use `dimension`.
const penpotTypes: Record<string, string | undefined> = {
  colors: "color",
  rounded: "borderRadius",
  spacing: "spacing",
  "x-sidereal.font-family": "fontFamilies",
  "x-sidereal.font-weight": "fontWeights",
  "x-sidereal.font-size": "fontSizes",
  "x-sidereal.line-height": "dimension",
  "x-sidereal.opacity": "opacity",
  "x-sidereal.blur": "dimension",
  "x-sidereal.border-width": "borderWidth",
  "x-sidereal.size": "sizing",
};

// Groups Penpot cannot hold. Its letter-spacing tokens hold pixels only, and
// DESIGN.md's letter spacing is in em.
const skippedGroups = ["x-sidereal.letter-spacing"];

function penpotValue(path: string, value: unknown): string {
  if (path === "x-sidereal.font-family") return firstFamily(String(value));
  // checkTokens() has already made sure pixel values are whole, such as 14px.
  return String(value).replace(/px$/, "");
}

// Penpot renders one family, so keep only the first. It splits an unquoted
// name at its spaces, so a name with spaces goes in single quotes.
function firstFamily(list: string): string {
  const name = list
    .split(",")[0]
    .trim()
    .replace(/^"(.*)"$/, "$1");
  return name.includes(" ") ? `'${name}'` : name;
}

// Runs main() when Node runs this file directly; see tokens.ts.
if (process.argv[1] === fileURLToPath(import.meta.url)) {
  process.exitCode = main();
}
