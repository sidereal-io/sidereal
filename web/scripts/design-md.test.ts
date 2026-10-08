// @vitest-environment node
import { describe, expect, it } from "vitest";
import { parseDesignMd, readDesignMd } from "./design-md.ts";

describe("parseDesignMd", () => {
  it("splits the front matter from the body", () => {
    const { frontMatter, body } = parseDesignMd(
      '---\ncolors:\n  white: "#FFFFFF"\n---\n\n# Title\n',
    );
    expect(frontMatter).toEqual({ colors: { white: "#FFFFFF" } });
    expect(body).toBe("\n# Title\n");
  });

  it("fails when there is no front matter", () => {
    expect(() => parseDesignMd("# Title\n")).toThrow(/no front matter/);
  });

  it("fails when the front matter is never closed", () => {
    expect(() => parseDesignMd("---\ncolors: {}\n")).toThrow(/no front matter/);
  });

  it("fails when the front matter is not a map", () => {
    expect(() => parseDesignMd("---\n- a list\n---\n")).toThrow(/YAML map/);
  });
});

describe("readDesignMd", () => {
  it("reads the repo's DESIGN.md", () => {
    const { frontMatter } = readDesignMd();
    expect(frontMatter).toHaveProperty("colors");
    expect(frontMatter).toHaveProperty("x-sidereal");
  });
});
