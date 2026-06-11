/**
 * Inverse of renderBody (body.ts): recover a bead's description and design from
 * a managed GitHub issue body. Strips the hidden marker and the derived
 * "### Blocked by" section. If the "## Design" heading is missing or mangled,
 * everything (minus blockers/marker) is the description and design is left
 * unspecified (undefined) so the caller leaves bead.design unchanged.
 */
const MARKER_RE = /<!--\s*managed by beads-sync[\s\S]*?-->/g;
// "### Blocked by" (rendered as the last section) through end of string.
const BLOCKED_RE = /\n#{2,3}\s+Blocked by[\s\S]*$/;
const DESIGN_HEADING_RE = /^##\s+Design\s*$/m;

export function parseBody(body: string | null | undefined): {
  description: string;
  design?: string;
} {
  let s = (body ?? '').replace(MARKER_RE, '');
  s = s.replace(BLOCKED_RE, '');

  const m = DESIGN_HEADING_RE.exec(s);
  if (!m) {
    return { description: s.trim() };
  }
  const description = s.slice(0, m.index).trim();
  const design = s.slice(m.index + m[0].length).trim();
  return { description, design };
}
