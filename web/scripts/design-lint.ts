// Lints DESIGN.md with the DESIGN.md specification's linter.
//
//   pnpm design:lint
//
// Fails on any error, and on any warning that is not a known one. The linter
// itself exits with status 0 on any number of warnings, so a new warning
// would otherwise pass unnoticed.
import { lint, type Finding } from "@google/design.md/linter";
import { readFileSync } from "node:fs";
import { designMdPath } from "./design-md.ts";

// Warnings DESIGN.md has on purpose, matched by rule and path.
export const knownWarnings = [
  // The palette has no single "primary" color: violet is split by use.
  { rule: "missing-primary", path: "colors" },
  // Type values are primitives under x-sidereal until text styles exist.
  { rule: "missing-typography", path: "typography" },
  // Sidereal's extension, which only web/scripts/tokens.ts reads.
  { rule: "token-like-ignored", path: "x-sidereal" },
];

export function main(): number {
  const unexpected = unexpectedFindings(
    lint(readFileSync(designMdPath, "utf8")).findings,
  );
  if (unexpected.length === 0) return 0;

  console.error("DESIGN.md has lint findings that are not known warnings:");
  for (const finding of unexpected) {
    const rule = finding.rule ? ` (${finding.rule})` : "";
    console.error(
      `  - ${finding.severity}${rule} at ${finding.path ?? "the file"}: ${finding.message}`,
    );
  }
  console.error(
    "Fix DESIGN.md. If a new warning is intended, add it to knownWarnings in web/scripts/design-lint.ts.",
  );
  return 1;
}

// Returns every error, and every warning not in knownWarnings.
// Info findings, such as the token summary, never fail the lint.
export function unexpectedFindings(findings: Finding[]): Finding[] {
  return findings.filter((finding) => {
    if (finding.severity === "error") return true;
    if (finding.severity !== "warning") return false;
    return !knownWarnings.some(
      (known) => known.rule === finding.rule && known.path === finding.path,
    );
  });
}

// Runs main() when Node runs this file directly.
if (import.meta.main) {
  process.exitCode = main();
}
