// @vitest-environment node
import { existsSync, readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { describe, expect, it } from "vitest";

const fontsCss = readFileSync(
  new URL("../src/styles/fonts.css", import.meta.url),
  "utf8",
);
const resolve = createRequire(import.meta.url).resolve;

// The package path inside each url("…") in fonts.css.
const fontUrls = [...fontsCss.matchAll(/url\("([^"]+)"\)/g)].map(
  (match) => match[1],
);

describe("fonts.css", () => {
  it("names both fonts", () => {
    expect(fontUrls).toHaveLength(2);
  });

  // A Fontsource update that renames a file fails here, instead of only in
  // the running app.
  it.each(fontUrls)("points at a file that exists: %s", (url) => {
    expect(existsSync(resolve(url))).toBe(true);
  });
});
