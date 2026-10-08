// @vitest-environment node
import { readdirSync, readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

interface Stylesheet {
  // The path a contributor would look for, such as src/styles/base.css.
  name: string;
  text: string;
}

// Lists each custom property a stylesheet uses through var() that no
// stylesheet defines, as "<property> in <stylesheet>". It checks every name,
// whatever its prefix, so a renamed prefix or a removed token fails here.
function undefinedProperties(stylesheets: Stylesheet[]): string[] {
  const defined = new Set(
    stylesheets.flatMap(({ text }) =>
      [...text.matchAll(/(--[\w-]+)\s*:/g)].map((match) => match[1]),
    ),
  );
  return stylesheets.flatMap(({ name, text }) =>
    [...text.matchAll(/var\(\s*(--[\w-]+)/g)]
      .map((match) => match[1])
      .filter((property) => !defined.has(property))
      .map((property) => `${property} in ${name}`),
  );
}

// Every .css file under web/src/.
function readStylesheets(): Stylesheet[] {
  const src = new URL("../src/", import.meta.url);
  return readdirSync(src, { recursive: true, encoding: "utf8" })
    .filter((path) => path.endsWith(".css"))
    .map((path) => ({
      name: `src/${path}`,
      text: readFileSync(new URL(path, src), "utf8"),
    }));
}

describe("the web shell's stylesheets", () => {
  it("include the token file and at least one stylesheet that uses it", () => {
    const names = readStylesheets().map(({ name }) => name);
    expect(names).toContain("src/styles/tokens.css");
    expect(names).toContain("src/styles/base.css");
  });

  it("use only custom properties that a stylesheet defines", () => {
    expect(undefinedProperties(readStylesheets())).toEqual([]);
  });
});

describe("undefinedProperties", () => {
  const base = {
    name: "base.css",
    text: "body { color: var(--sr-obsidian-100); }",
  };

  it("finds a property after the prefix changes", () => {
    const tokens = {
      name: "tokens.css",
      text: ":root { --foo-obsidian-100: #F5F4EF; }",
    };
    expect(undefinedProperties([tokens, base])).toEqual([
      "--sr-obsidian-100 in base.css",
    ]);
  });

  it("finds a removed token that a stylesheet still uses", () => {
    const tokens = {
      name: "tokens.css",
      text: ":root { --sr-obsidian-950: #0B0A0F; }",
    };
    expect(undefinedProperties([tokens, base])).toEqual([
      "--sr-obsidian-100 in base.css",
    ]);
  });

  it("accepts a property a stylesheet defines for itself", () => {
    const local = {
      name: "panel.css",
      text: ".panel { --gap: 4px; padding: var(--gap); }",
    };
    expect(undefinedProperties([local])).toEqual([]);
  });
});
