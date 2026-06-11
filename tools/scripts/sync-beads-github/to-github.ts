// beads -> GitHub field transforms: render a bead into the title/body/labels/
// state/external-ref a GitHub issue should carry. The inverse lives in
// `from-github.ts`.
import type { Bead } from './beads.ts';

// --- managed body marker ---------------------------------------------------

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

// --- labels ----------------------------------------------------------------

const PRIORITY_NAMES = ['p0', 'p1', 'p2', 'p3', 'p4'];

/** Label prefixes the sync owns — current slash form plus legacy double-colon. */
export const MANAGED_PREFIXES = [
  'type/',
  'priority/',
  'status/',
  'type::',
  'priority::',
  'status::',
];
export const BLOCKED_LABEL = 'blocked';

export function priorityLabel(priority: number): string {
  const name = PRIORITY_NAMES[priority] ?? `p${priority}`;
  return `priority/${name}`;
}

/** The labels the bead should produce on GitHub. */
export function computeManagedLabels(bead: Bead, hasOpenBlocker: boolean): string[] {
  const out = new Set<string>();
  out.add(`type/${bead.issue_type}`);
  out.add(priorityLabel(bead.priority));
  out.add(`status/${bead.status}`);
  for (const l of bead.labels ?? []) out.add(l);
  if (hasOpenBlocker) out.add(BLOCKED_LABEL);
  return [...out];
}

function isManaged(label: string, beadLabels: Set<string>): boolean {
  if (label === BLOCKED_LABEL) return true;
  if (MANAGED_PREFIXES.some((p) => label.startsWith(p))) return true;
  return beadLabels.has(label); // bd's own free-form labels are managed too
}

/**
 * Final label set: keep human-added labels, drop stale labels in the managed
 * namespace, then add the desired managed labels.
 *
 * Known limitation: a free-form bd label that was *removed* from the bead is
 * indistinguishable from a human-added label, so it lingers. Acceptable.
 */
export function reconcileLabels(
  existing: string[],
  desired: string[],
  beadLabels: string[],
): string[] {
  const beadSet = new Set(beadLabels);
  const desiredSet = new Set(desired);
  const result = new Set<string>();
  for (const l of existing) {
    if (isManaged(l, beadSet) && !desiredSet.has(l)) continue; // stale managed
    result.add(l);
  }
  for (const l of desired) result.add(l);
  return [...result];
}

// --- state -----------------------------------------------------------------

export type GithubState = 'open' | 'closed';

/**
 * The GitHub issue state that mirrors this bead. One-directional: the push
 * sends this to GitHub and never reads GitHub state back into the bead.
 */
export function githubStateForBead(bead: Bead): GithubState {
  return bead.status === 'closed' ? 'closed' : 'open';
}

// --- external ref ----------------------------------------------------------

/** Canonical external_ref URL for a given issue number. `repo` is "owner/name". */
export function refUrlForNumber(repo: string, issueNumber: number): string {
  return `https://github.com/${repo}/issues/${issueNumber}`;
}
