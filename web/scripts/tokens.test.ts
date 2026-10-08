// @vitest-environment node
import { describe, expect, it } from "vitest";
import { isMap, readDesignMd } from "./design-md.ts";
import { buildTokensCss } from "./tokens.ts";

const { frontMatter } = readDesignMd();

// Every `--name: value;` line in the generated CSS, in order.
function properties(css: string): [string, string][] {
  return [...css.matchAll(/^\s*(--[\w-]+):\s*(.+);$/gm)].map((match) => [
    match[1],
    match[2],
  ]);
}

// The keys of every primitive group in the front matter.
function primitiveKeys(): string[] {
  const groups = [frontMatter.colors, frontMatter.rounded, frontMatter.spacing];
  const extension = frontMatter["x-sidereal"];
  if (isMap(extension)) {
    for (const [name, group] of Object.entries(extension)) {
      if (name !== "css-prefix" && name !== "semantic") groups.push(group);
    }
  }
  return groups.flatMap((group) => (isMap(group) ? Object.keys(group) : []));
}

describe("buildTokensCss on DESIGN.md", () => {
  const css = buildTokensCss(frontMatter);
  const byName = new Map(properties(css));

  it("writes a color token", () => {
    expect(byName.get("--sr-obsidian-950")).toBe("#0B0A0F");
  });

  it("writes tokens from the extension", () => {
    expect(byName.get("--sr-font-size-14")).toBe("14px");
    expect(byName.get("--sr-opacity-45")).toBe("0.45");
    expect(byName.get("--sr-font-family-mono")).toBe(
      '"Atkinson Hyperlegible Mono", ui-monospace, "SF Mono", Menlo, monospace',
    );
  });

  it("writes one property per primitive, and nothing else", () => {
    const names = properties(css).map(([name]) => name);
    const expected = primitiveKeys().map((key) => `--sr-${key}`);
    expect(names).toEqual(expected);
  });

  it("starts with a comment that says the file is generated", () => {
    expect(css.startsWith("/* Generated from DESIGN.md")).toBe(true);
  });
});

describe("buildTokensCss", () => {
  it("writes a new token", () => {
    const css = buildTokensCss({
      spacing: { "spacing-16": "16px" },
      "x-sidereal": { "css-prefix": "sr" },
    });
    expect(new Map(properties(css)).get("--sr-spacing-16")).toBe("16px");
  });

  it("uses the prefix from x-sidereal.css-prefix", () => {
    const css = buildTokensCss({
      spacing: { "spacing-4": "4px" },
      "x-sidereal": { "css-prefix": "foo" },
    });
    expect(new Map(properties(css)).get("--foo-spacing-4")).toBe("4px");
  });
});
