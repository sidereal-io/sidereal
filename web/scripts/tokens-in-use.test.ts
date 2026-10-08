// @vitest-environment node
import { readdirSync, readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

interface SourceFile {
  // The path a contributor would look for, such as src/styles/base.css.
  name: string;
  text: string;
}

// Lists each custom property a source file uses through var() that no
// stylesheet defines, as "<property> in <file>". It checks every name,
// whatever its prefix, so a renamed prefix or a removed token fails here.
function undefinedProperties(files: SourceFile[]): string[] {
  const defined = new Set(
    files.flatMap(({ text }) =>
      [...text.matchAll(/(--[\w-]+)\s*:/g)].map((match) => match[1]),
    ),
  );
  return files.flatMap(({ name, text }) =>
    [...text.matchAll(/var\(\s*(--[\w-]+)/g)]
      .map((match) => match[1])
      .filter((property) => !defined.has(property))
      .map((property) => `${property} in ${name}`),
  );
}

// Every stylesheet and TypeScript file under web/src/. Components may use a
// token through var() in an inline style, so they are checked too.
function readSourceFiles(): SourceFile[] {
  const src = new URL("../src/", import.meta.url);
  return readdirSync(src, { recursive: true, encoding: "utf8" })
    .filter((path) => /\.(css|ts|tsx)$/.test(path))
    .map((path) => ({
      name: `src/${path}`,
      text: readFileSync(new URL(path, src), "utf8"),
    }));
}

describe("the web shell's source files", () => {
  it("include the token file and at least one stylesheet that uses it", () => {
    const names = readSourceFiles().map(({ name }) => name);
    expect(names).toContain("src/styles/tokens.css");
    expect(names).toContain("src/styles/base.css");
  });

  it("use only custom properties that a stylesheet defines", () => {
    expect(undefinedProperties(readSourceFiles())).toEqual([]);
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

  it("finds a token used in a component's inline style", () => {
    const tokens = {
      name: "tokens.css",
      text: ":root { --sr-obsidian-950: #0B0A0F; }",
    };
    const component = {
      name: "Panel.tsx",
      text: 'const style = { color: "var(--sr-obsidian-100)" };',
    };
    expect(undefinedProperties([tokens, component])).toEqual([
      "--sr-obsidian-100 in Panel.tsx",
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
