import type { Bead } from './bead.ts';

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
