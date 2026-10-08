// @vitest-environment node
import { describe, expect, it } from "vitest";
import { isMap, readDesignMd } from "./design-md.ts";
import { type PenpotToken, buildPenpotTokens } from "./penpot-tokens.ts";
import { TokenError } from "./tokens.ts";

const { frontMatter } = readDesignMd();

function byName(tokens: PenpotToken[]) {
  return new Map(tokens.map((token) => [token.name, token]));
}

describe("buildPenpotTokens on DESIGN.md", () => {
  const tokens = buildPenpotTokens(frontMatter);
  const named = byName(tokens);

  it("writes one token per primitive, in order, except letter spacing", () => {
    const extension = frontMatter["x-sidereal"];
    const letterSpacing =
      isMap(extension) && isMap(extension["letter-spacing"])
        ? Object.keys(extension["letter-spacing"])
        : [];
    expect(letterSpacing.length).toBeGreaterThan(0);
    for (const key of letterSpacing) expect(named.has(key)).toBe(false);
    expect(new Set(tokens.map((token) => token.name)).size).toBe(tokens.length);
  });

  it("writes the tokens Penpot was missing", () => {
    expect(named.get("border-width-1")).toEqual({
      group: "border-width",
      type: "borderWidth",
      name: "border-width-1",
      value: "1",
    });
    expect(named.get("size-32")).toEqual({
      group: "size",
      type: "sizing",
      name: "size-32",
      value: "32",
    });
  });

  it("quotes the first mono family, so Penpot does not split it", () => {
    expect(named.get("font-family-mono")?.value).toBe(
      "'Atkinson Hyperlegible Mono'",
    );
    expect(named.get("font-family-sans")?.value).toBe("Inter");
  });
});

describe("buildPenpotTokens", () => {
  const base = { "css-prefix": "sr" };

  it("maps each group to a Penpot type, names the group, and drops px", () => {
    const tokens = buildPenpotTokens({
      colors: { "obsidian-950": "#0B0A0F" },
      rounded: { "radius-full": "9999px" },
      spacing: { "spacing-8": "8px" },
      "x-sidereal": {
        ...base,
        "font-weight": { "font-weight-600": 600 },
        "font-size": { "font-size-14": "14px" },
        "line-height": { "line-height-20": "20px" },
        opacity: { "opacity-45": 0.45 },
        blur: { "blur-8": "8px" },
      },
    });
    expect(tokens).toEqual([
      {
        group: "colors",
        type: "color",
        name: "obsidian-950",
        value: "#0B0A0F",
      },
      {
        group: "rounded",
        type: "borderRadius",
        name: "radius-full",
        value: "9999",
      },
      { group: "spacing", type: "spacing", name: "spacing-8", value: "8" },
      {
        group: "font-weight",
        type: "fontWeights",
        name: "font-weight-600",
        value: "600",
      },
      {
        group: "font-size",
        type: "fontSizes",
        name: "font-size-14",
        value: "14",
      },
      {
        group: "line-height",
        type: "dimension",
        name: "line-height-20",
        value: "20",
      },
      { group: "opacity", type: "opacity", name: "opacity-45", value: "0.45" },
      { group: "blur", type: "dimension", name: "blur-8", value: "8" },
    ]);
  });

  it("keeps an unquoted single-word family as it is", () => {
    const [token] = buildPenpotTokens({
      "x-sidereal": {
        ...base,
        "font-family": { "font-family-sans": "Inter, sans-serif" },
      },
    });
    expect(token.value).toBe("Inter");
  });

  it("quotes an unquoted family with spaces", () => {
    const [token] = buildPenpotTokens({
      "x-sidereal": {
        ...base,
        "font-family": { "font-family-ui": "Segoe UI, sans-serif" },
      },
    });
    expect(token.value).toBe("'Segoe UI'");
  });

  it("refuses the same tokens that tokens.css refuses", () => {
    expect(() =>
      buildPenpotTokens({ colors: { red: "red" }, "x-sidereal": base }),
    ).toThrow(TokenError);
  });
});
