// @vitest-environment node
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { checkContrastTable, contrastRatio } from "./contrast.ts";
import { designMdPath } from "./design-md.ts";

const designMd = readFileSync(designMdPath, "utf8");

// Changes one exact piece of DESIGN.md, and fails if it is not there.
function edit(find: string, replace: string): string {
  expect(designMd).toContain(find);
  return designMd.replace(find, replace);
}

describe("DESIGN.md's contrast table", () => {
  it("holds only true claims", () => {
    expect(checkContrastTable(designMd)).toEqual([]);
  });
});

describe("checkContrastTable", () => {
  it("fails when a color changes but its claims do not", () => {
    const text = edit('obsidian-500: "#837F90"', 'obsidian-500: "#6F6B7C"');
    expect(checkContrastTable(text).join("\n")).toMatch(
      /obsidian-500 on obsidian-950/,
    );
  });

  it("fails when a claimed ratio is wrong", () => {
    const text = edit(
      "| `obsidian-400` text | 7.2 |",
      "| `obsidian-400` text | 7.5 |",
    );
    expect(checkContrastTable(text)).toEqual([
      "obsidian-400 on obsidian-950: the table says 7.5, but the ratio is 7.17",
    ]);
  });

  it("fails when a cell holds no claim it can read", () => {
    const text = edit("| 4.8 | 4.5 |", "| 4.8 | TBD |");
    expect(checkContrastTable(text).join("\n")).toMatch(
      /obsidian-500 on obsidian-850: a cell must hold/,
    );
  });

  it("fails when a failing pair is not marked", () => {
    const text = edit(
      "| `obsidian-600` text | 3.2 (fails) |",
      "| `obsidian-600` text | 3.2 |",
    );
    expect(checkContrastTable(text).join("\n")).toMatch(
      /obsidian-600 on obsidian-950: 3\.19 is below 4\.5:1/,
    );
  });

  it("fails when a passing pair is marked as failing", () => {
    const text = edit(
      "| `obsidian-400` text | 7.2 |",
      "| `obsidian-400` text | 7.2 (fails) |",
    );
    expect(checkContrastTable(text).join("\n")).toMatch(
      /obsidian-400 on obsidian-950: marked \(fails\)/,
    );
  });

  it("fails when the table has no rows", () => {
    const rows = designMd.match(
      /^\| `obsidian-\d+` text .*\n|^\| `violet-500` text .*\n/gm,
    );
    expect(rows).toHaveLength(4 + 1);
    let text = designMd;
    for (const row of rows ?? []) text = text.replace(row, "");
    expect(checkContrastTable(text)).toEqual([
      "the contrast table has no rows",
    ]);
  });

  it("checks a row written without outer pipes", () => {
    const text = edit(
      "| `obsidian-400` text | 7.2 | 6.8 | 6.4 |",
      "`obsidian-400` text | 7.5 | 6.8 | 6.4",
    );
    expect(checkContrastTable(text)).toEqual([
      "obsidian-400 on obsidian-950: the table says 7.5, but the ratio is 7.17",
    ]);
  });

  it("reads DESIGN.md with CRLF line endings", () => {
    expect(checkContrastTable(designMd.replaceAll("\n", "\r\n"))).toEqual([]);
  });

  it("fails when the table is missing", () => {
    const text = edit("### Contrast", "### Contrast notes");
    expect(checkContrastTable(text)).toEqual([
      "DESIGN.md has no table under its `### Contrast` heading",
    ]);
  });

  it("fails when a name is not a color token", () => {
    const text = edit("| `violet-500` text |", "| `violet-550` text |");
    expect(checkContrastTable(text).join("\n")).toMatch(
      /violet-550 on obsidian-950: both names must be color tokens/,
    );
  });
});

describe("contrastRatio", () => {
  it("matches the WCAG 2 extremes", () => {
    expect(contrastRatio("#FFFFFF", "#000000")).toBeCloseTo(21, 5);
    expect(contrastRatio("#777777", "#777777")).toBe(1);
  });

  it("does not depend on the order of the colors", () => {
    expect(contrastRatio("#0B0A0F", "#F5F4EF")).toBe(
      contrastRatio("#F5F4EF", "#0B0A0F"),
    );
  });
});
