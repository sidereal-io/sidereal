import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { parse } from "yaml";

export interface DesignMd {
  // The YAML between the opening and closing `---` lines.
  frontMatter: Record<string, unknown>;
  // Everything after the closing `---` line: the Markdown prose.
  body: string;
}

// DESIGN.md sits at the repo root, one level above web/.
export const designMdPath = fileURLToPath(
  new URL("../../DESIGN.md", import.meta.url),
);

// Reads DESIGN.md and splits it into front matter and body.
// Throws when the file has no front matter, or when it is not a YAML map.
export function readDesignMd(path: string = designMdPath): DesignMd {
  return parseDesignMd(readFileSync(path, "utf8"));
}

export function parseDesignMd(text: string): DesignMd {
  const lines = text.split("\n");
  const closing = lines.indexOf("---", 1);
  if (lines[0] !== "---" || closing === -1) {
    throw new Error(
      "DESIGN.md has no front matter: it must start with a `---` line and close it with another",
    );
  }

  const frontMatter: unknown = parse(lines.slice(1, closing).join("\n"));
  if (!isMap(frontMatter)) {
    throw new Error("DESIGN.md's front matter must be a YAML map of keys");
  }

  return { frontMatter, body: lines.slice(closing + 1).join("\n") };
}

export function isMap(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
