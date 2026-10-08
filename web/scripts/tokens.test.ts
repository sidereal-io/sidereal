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

describe("buildTokensCss refuses tokens it cannot write", () => {
  // A small, valid front matter that each case changes in one place.
  function base(): Record<string, Record<string, unknown>> {
    return {
      colors: { white: "#FFFFFF" },
      rounded: { "radius-4": "4px" },
      spacing: { "spacing-4": "4px" },
      "x-sidereal": {
        "css-prefix": "sr",
        "font-family": {
          "font-family-sans": 'Inter, -apple-system, "Segoe UI"',
        },
        "font-weight": { "font-weight-400": 400 },
        "letter-spacing": { "letter-spacing-n015": "-0.015em" },
        opacity: { "opacity-45": 0.45 },
        size: { "size-1280": "1280px" },
        semantic: {},
      },
      components: {},
    };
  }

  it("accepts the base front matter", () => {
    expect(() => buildTokensCss(base())).not.toThrow();
  });

  it("refuses a semantic token", () => {
    const frontMatter = base();
    frontMatter["x-sidereal"].semantic = {
      "color-action-primary": "{colors.violet-600}",
    };
    expect(() => buildTokensCss(frontMatter)).toThrow(/color-action-primary/);
  });

  it("refuses a component token", () => {
    const frontMatter = base();
    frontMatter.components = {
      "button-primary-hover": { backgroundColor: "{colors.violet-500}" },
    };
    expect(() => buildTokensCss(frontMatter)).toThrow(/button-primary-hover/);
  });

  it("refuses a key used in two groups", () => {
    const frontMatter = base();
    frontMatter["x-sidereal"].size = { "spacing-4": "4px" };
    expect(() => buildTokensCss(frontMatter)).toThrow(/spacing-4/);
  });

  it("refuses a key with characters other than a-z, 0-9, and hyphens", () => {
    const frontMatter = base();
    frontMatter.spacing = { Spacing_4: "4px" };
    expect(() => buildTokensCss(frontMatter)).toThrow(/Spacing_4/);
  });

  it("refuses a prefix with characters other than a-z", () => {
    const frontMatter = base();
    frontMatter["x-sidereal"]["css-prefix"] = "sr; } body { color: red } /*";
    expect(() => buildTokensCss(frontMatter)).toThrow(/css-prefix/);
  });

  it.each([";", "{", "}", "<", "\\", "\n", "url("])(
    "refuses a value that holds %j",
    (text) => {
      const frontMatter = base();
      frontMatter["x-sidereal"]["font-family"] = {
        "font-family-sans": `Inter${text}x`,
      };
      expect(() => buildTokensCss(frontMatter)).toThrow(/font-family-sans/);
    },
  );

  it("refuses a value that would escape its property", () => {
    const frontMatter = base();
    frontMatter["x-sidereal"].size = {
      "size-1280": "1px; } body { background: url(https://x.invalid) }",
    };
    expect(() => buildTokensCss(frontMatter)).toThrow(/size-1280/);
  });

  it.each([
    ["colors", "white", "#FFF"],
    ["colors", "white", "white"],
    ["rounded", "radius-4", "4"],
    ["spacing", "spacing-4", "4.5px"],
    ["x-sidereal.size", "size-1280", "80rem"],
    ["x-sidereal.letter-spacing", "letter-spacing-n015", "-0.015px"],
    ["x-sidereal.opacity", "opacity-45", 1.5],
    ["x-sidereal.opacity", "opacity-45", "0.45"],
    ["x-sidereal.font-weight", "font-weight-400", 1001],
    ["x-sidereal.font-weight", "font-weight-400", 400.5],
    ["x-sidereal.font-family", "font-family-sans", "12px"],
    ["x-sidereal.font-family", "font-family-sans", "Inter, 'Segoe UI'"],
  ])("refuses %s: %s = %j", (path, key, value) => {
    const frontMatter = base();
    const group = path.startsWith("x-sidereal.")
      ? (frontMatter["x-sidereal"][path.slice("x-sidereal.".length)] as Record<
          string,
          unknown
        >)
      : frontMatter[path];
    group[key] = value;
    expect(() => buildTokensCss(frontMatter)).toThrow(new RegExp(key));
  });

  it("refuses a missing prefix", () => {
    const frontMatter = base();
    delete frontMatter["x-sidereal"]["css-prefix"];
    expect(() => buildTokensCss(frontMatter)).toThrow(/css-prefix/);
  });

  it("refuses typography text styles", () => {
    const frontMatter = base();
    frontMatter.typography = { "body-md": { fontSize: "14px" } };
    expect(() => buildTokensCss(frontMatter)).toThrow(/typography/);
  });

  it("accepts an empty typography group", () => {
    const frontMatter = base();
    frontMatter.typography = {};
    expect(() => buildTokensCss(frontMatter)).not.toThrow();
  });

  it("refuses a spec group that is not a map", () => {
    const frontMatter: Record<string, unknown> = base();
    frontMatter.colors = ["#000000"];
    expect(() => buildTokensCss(frontMatter)).toThrow(/colors must be a map/);
  });

  it.each([
    "inherit",
    "initial",
    "unset",
    "revert",
    "revert-layer",
    "Inter, INHERIT",
  ])("refuses the CSS-wide keyword font list %j", (value) => {
    const frontMatter = base();
    frontMatter["x-sidereal"]["font-family"] = { "font-family-sans": value };
    expect(() => buildTokensCss(frontMatter)).toThrow(/font-family-sans/);
  });

  it("refuses an x-sidereal group with no known form", () => {
    const frontMatter = base();
    frontMatter["x-sidereal"].shadow = { "shadow-1": "0 0 4px" };
    expect(() => buildTokensCss(frontMatter)).toThrow(/shadow/);
  });

  it("names every problem at once", () => {
    const frontMatter = base();
    frontMatter.spacing = { "spacing-4": "four" };
    frontMatter.colors = { white: "#FFF" };
    expect(() => buildTokensCss(frontMatter)).toThrow(/white[\s\S]*spacing-4/);
  });
});
