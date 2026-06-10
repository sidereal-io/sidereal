import type { Bead } from './bead.ts';

/** Hidden marker carrying the bead id, used for idempotent matching. */
export function markerFor(beadId: string): string {
  return `<!-- managed by beads-sync · ${beadId} · do not edit -->`;
}

const MARKER_RE = /<!--\s*managed by beads-sync · (\S+) · do not edit\s*-->/;

export function extractMarkerId(body: string | null | undefined): string | null {
  if (!body) return null;
  const m = MARKER_RE.exec(body);
  return m ? m[1] : null;
}

/** Render the fully beads-managed issue body. */
export function renderBody(bead: Bead, blockerIssueNumbers: number[]): string {
  const parts: string[] = [];
  const desc = (bead.description ?? '').trim();
  if (desc) parts.push(desc);
  if (bead.design && bead.design.trim()) {
    parts.push(`## Design\n\n${bead.design.trim()}`);
  }
  if (blockerIssueNumbers.length > 0) {
    const list = blockerIssueNumbers.map((n) => `- #${n}`).join('\n');
    parts.push(`### Blocked by\n\n${list}`);
  }
  parts.push(markerFor(bead.id));
  return parts.join('\n\n') + '\n';
}
