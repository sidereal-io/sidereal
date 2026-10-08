// @vitest-environment node
import { lint } from "@google/design.md/linter";
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { designMdPath } from "./design-md.ts";
import { unexpectedFindings } from "./design-lint.ts";

const designMd = readFileSync(designMdPath, "utf8");

function unexpectedIn(text: string) {
  return unexpectedFindings(lint(text).findings);
}

describe("unexpectedFindings", () => {
  it("finds nothing unexpected in DESIGN.md", () => {
    expect(unexpectedIn(designMd)).toEqual([]);
  });

  it("reports a new warning, even one with a known rule", () => {
    const text = designMd.replace(
      "\ncomponents: {}\n",
      "\ncomponents: {}\nx-other:\n  thing-4: 4px\n",
    );
    expect(text).not.toBe(designMd);
    const findings = unexpectedIn(text);
    expect(findings).toHaveLength(1);
    expect(findings[0]).toMatchObject({ severity: "warning", path: "x-other" });
  });

  it("reports an error", () => {
    const text = designMd.replace(
      'obsidian-950: "#0B0A0F"',
      'obsidian-950: "#GGGGGG"',
    );
    expect(text).not.toBe(designMd);
    expect(unexpectedIn(text)).toContainEqual(
      expect.objectContaining({
        severity: "error",
        path: "colors.obsidian-950",
      }),
    );
  });
});
