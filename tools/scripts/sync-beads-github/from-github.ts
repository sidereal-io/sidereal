// GitHub -> beads field transforms: parse an issue's body/labels/external-ref
// back into bead fields. The inverse lives in `to-github.ts`.

// --- body ------------------------------------------------------------------

/**
 * Inverse of renderBody (to-github.ts): recover a bead's description and design
 * from a managed GitHub issue body. Strips the hidden marker and the derived
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

// --- labels ----------------------------------------------------------------

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

// --- external ref ----------------------------------------------------------

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
