// Pure field transforms between a Bead and its GitHub issue representation.
// No I/O. Two directions: beads -> GitHub (render/compute) and GitHub -> beads
// (parse). The orchestrators (push.ts / pull.ts) each use what they need.
import type { Bead } from './beads.ts';

// ===========================================================================
// beads -> GitHub
// ===========================================================================

// --- managed body marker ---------------------------------------------------

/** Hidden marker carrying the bead id, used for idempotent matching. */
export function markerFor(beadId: string): string {
  return `<!-- managed by beads-sync · ${beadId} · do not edit -->`;
}

const MARKER_ID_RE = /<!--\s*managed by beads-sync · (\S+) · do not edit\s*-->/;

export function extractMarkerId(body: string | null | undefined): string | null {
  if (!body) return null;
  const m = MARKER_ID_RE.exec(body);
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

// --- labels (compute) ------------------------------------------------------

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

// --- external ref (build) --------------------------------------------------

/** Canonical external_ref URL for a given issue number. `repo` is "owner/name". */
export function refUrlForNumber(repo: string, issueNumber: number): string {
  return `https://github.com/${repo}/issues/${issueNumber}`;
}

// ===========================================================================
// GitHub -> beads
// ===========================================================================

// --- body (parse) ----------------------------------------------------------

/**
 * Inverse of renderBody: recover a bead's description and design from a managed
 * GitHub issue body. Strips the hidden marker and the derived "### Blocked by"
 * section. If the "## Design" heading is missing or mangled, everything (minus
 * blockers/marker) is the description and design is left unspecified (undefined)
 * so the caller leaves bead.design unchanged.
 */
const MARKER_STRIP_RE = /<!--\s*managed by beads-sync[\s\S]*?-->/g;
// "### Blocked by" (rendered as the last section) through end of string.
const BLOCKED_RE = /\n#{2,3}\s+Blocked by[\s\S]*$/;
const DESIGN_HEADING_RE = /^##\s+Design\s*$/m;

export function parseBody(body: string | null | undefined): {
  description: string;
  design?: string;
} {
  let s = (body ?? '').replace(MARKER_STRIP_RE, '');
  s = s.replace(BLOCKED_RE, '');

  const m = DESIGN_HEADING_RE.exec(s);
  if (!m) {
    return { description: s.trim() };
  }
  const description = s.slice(0, m.index).trim();
  const design = s.slice(m.index + m[0].length).trim();
  return { description, design };
}

// --- labels (parse) --------------------------------------------------------

const VALID_TYPES = new Set(['bug', 'feature', 'task', 'epic', 'chore', 'decision']);
const VALID_STATUSES = new Set([
  'open',
  'in_progress',
  'blocked',
  'deferred',
  'closed',
  'pinned',
  'hooked',
]);

export interface SplitLabels {
  freeform: string[];
  issueType?: string;
  priority?: number;
  status?: string;
  warnings: string[];
}

/**
 * Split GitHub labels into free-form labels (-> bead labels) and structured
 * fields (type/priority/status). The track label and the derived "blocked"
 * label are indicators, never data. Invalid structured values are dropped and
 * reported in `warnings`.
 */
export function splitLabels(labels: string[], track: string): SplitLabels {
  const freeform: string[] = [];
  const warnings: string[] = [];
  let issueType: string | undefined;
  let priority: number | undefined;
  let status: string | undefined;

  for (const l of labels) {
    if (l === track || l === 'blocked') continue; // indicators
    if (l.startsWith('type/')) {
      const v = l.slice('type/'.length);
      if (VALID_TYPES.has(v)) issueType = v;
      else warnings.push(`ignoring invalid type label "${l}"`);
    } else if (l.startsWith('priority/')) {
      const m = /^p([0-4])$/.exec(l.slice('priority/'.length));
      if (m) priority = Number(m[1]);
      else warnings.push(`ignoring invalid priority label "${l}"`);
    } else if (l.startsWith('status/')) {
      const v = l.slice('status/'.length);
      if (VALID_STATUSES.has(v)) status = v;
      else warnings.push(`ignoring invalid status label "${l}"`);
    } else {
      freeform.push(l);
    }
  }
  return { freeform, issueType, priority, status, warnings };
}

// --- external ref (parse) --------------------------------------------------

const ISSUE_URL_RE = /\/issues\/(\d+)\/?$/;

/** Extract a GitHub issue number from a bead external_ref (URL or bare number). */
export function issueNumberFromRef(ref: string | undefined | null): number | null {
  if (!ref) return null;
  const trimmed = ref.trim();
  const m = ISSUE_URL_RE.exec(trimmed);
  if (m) return Number(m[1]);
  const n = Number(trimmed);
  return Number.isInteger(n) && n > 0 ? n : null;
}
