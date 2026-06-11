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
