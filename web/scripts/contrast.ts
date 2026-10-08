import { isMap, parseDesignMd } from "./design-md.ts";

// Every pair in DESIGN.md's contrast table is body text, which needs 4.5:1.
export const textContrastMinimum = 4.5;

// Checks every claim in the table under DESIGN.md's `### Contrast` heading
// against the colors in its front matter. Returns one line per problem; an
// empty list means every claim holds.
export function checkContrastTable(designMd: string): string[] {
  const { frontMatter, body } = parseDesignMd(designMd);
  const colors = isMap(frontMatter.colors) ? frontMatter.colors : {};
  const table = findContrastTable(body);
  if (table === undefined) {
    return ["DESIGN.md has no table under its `### Contrast` heading"];
  }

  const [header, ...rows] = table;
  const backgrounds = header.slice(1).map(tokenIn);
  const problems: string[] = [];
  if (backgrounds.length === 0) {
    problems.push("the contrast table has no background columns");
  }
  if (rows.length === 0) problems.push("the contrast table has no rows");

  for (const row of rows) {
    const text = tokenIn(row.at(0) ?? "");
    for (const [column, background] of backgrounds.entries()) {
      // A short row has no cell here, which checkCell reports.
      const cell = (row.at(column + 1) ?? "").trim();
      const pair = `${text ?? row.at(0)?.trim() ?? "?"} on ${background ?? "?"}`;
      problems.push(...checkCell(pair, cell, text, background, colors));
    }
  }
  return problems;
}

// A claim: a ratio with one decimal place, optionally marked `(fails)`.
const claimPattern = /^(\d+\.\d)( \(fails\))?$/;

function checkCell(
  pair: string,
  cell: string,
  text: string | undefined,
  background: string | undefined,
  colors: Record<string, unknown>,
): string[] {
  if (cell === "—") return [];

  const claim = claimPattern.exec(cell);
  if (claim === null) {
    return [
      `${pair}: a cell must hold a ratio such as 4.5, a ratio followed by (fails), or —`,
    ];
  }

  const textColor = text === undefined ? undefined : colors[text];
  const backgroundColor =
    background === undefined ? undefined : colors[background];
  if (typeof textColor !== "string" || typeof backgroundColor !== "string") {
    return [`${pair}: both names must be color tokens in the front matter`];
  }

  const ratio = contrastRatio(textColor, backgroundColor);
  const claimed = Number(claim[1]);
  const markedFails = claim[2] === " (fails)";
  const problems: string[] = [];
  if (Math.round(ratio * 10) !== Math.round(claimed * 10)) {
    problems.push(
      `${pair}: the table says ${claim[1]}, but the ratio is ${ratio.toFixed(2)}`,
    );
  }
  if (markedFails && ratio >= textContrastMinimum) {
    problems.push(
      `${pair}: marked (fails), but ${ratio.toFixed(2)} meets ${String(textContrastMinimum)}:1`,
    );
  }
  if (!markedFails && ratio < textContrastMinimum) {
    problems.push(
      `${pair}: ${ratio.toFixed(2)} is below ${String(textContrastMinimum)}:1, so the cell must be marked (fails)`,
    );
  }
  return problems;
}

// The WCAG 2 contrast ratio between two `#RRGGBB` colors.
export function contrastRatio(first: string, second: string): number {
  const [lighter, darker] = [
    relativeLuminance(first),
    relativeLuminance(second),
  ].sort((a, b) => b - a) as [number, number];
  return (lighter + 0.05) / (darker + 0.05);
}

function relativeLuminance(hex: string): number {
  const [red, green, blue] = [1, 3, 5].map((start) => {
    const channel = parseInt(hex.slice(start, start + 2), 16) / 255;
    return channel <= 0.04045
      ? channel / 12.92
      : ((channel + 0.055) / 1.055) ** 2.4;
  }) as [number, number, number];
  return 0.2126 * red + 0.7152 * green + 0.0722 * blue;
}

// Returns the cells of the first Markdown table under `### Contrast`, without
// its separator row. Returns undefined when there is no such table.
function findContrastTable(
  body: string,
): [string[], ...string[][]] | undefined {
  const lines = body.split(/\r?\n/);
  const heading = lines.findIndex((line) => line.trim() === "### Contrast");
  if (heading === -1) return undefined;

  const table: string[][] = [];
  for (const line of lines.slice(heading + 1)) {
    const trimmed = line.trim();
    if (trimmed.startsWith("#")) break;
    // A table starts at a line with a leading pipe. After that, a row may
    // leave out its outer pipes, as Markdown allows.
    const isRow =
      trimmed.startsWith("|") || (table.length > 0 && trimmed.includes("|"));
    if (!isRow) {
      if (table.length > 0) break;
      continue;
    }
    const cells = trimmed.replace(/^\|/, "").replace(/\|$/, "").split("|");
    if (cells.every((cell) => /^\s*:?-+:?\s*$/.test(cell))) continue;
    table.push(cells);
  }
  const header = table.shift();
  return header === undefined ? undefined : [header, ...table];
}

// The token name in a cell such as "On `obsidian-950`".
function tokenIn(cell: string): string | undefined {
  return /`([a-z0-9-]+)`/.exec(cell)?.[1];
}
