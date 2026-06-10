export type DependencyType = 'parent-child' | 'blocks' | 'related' | 'discovered-from';

export interface BeadDependency {
  id: string;
  dependency_type: DependencyType;
  status?: string;
  external_ref?: string;
}

export interface Bead {
  id: string;
  title: string;
  description?: string;
  design?: string;
  status: string; // open | in_progress | blocked | closed | ...
  priority: number; // 0..4
  issue_type: string; // task | bug | feature | epic
  owner?: string; // assignee
  labels?: string[];
  parent?: string;
  external_ref?: string;
  updated_at: string;
  dependencies?: BeadDependency[];
}

/** Parse the output of `bd list --json`. */
export function parseBeads(json: string): Bead[] {
  const data = JSON.parse(json);
  if (!Array.isArray(data)) {
    throw new Error('expected `bd list --json` to return an array');
  }
  return data as Bead[];
}

/** IDs of beads that block this bead (rendered as "Blocked by"). */
export function blockerBeadIds(bead: Bead): string[] {
  return (bead.dependencies ?? [])
    .filter((d) => d.dependency_type === 'blocks')
    .map((d) => d.id);
}

/** The parent bead id, from the top-level field or a parent-child dependency. */
export function parentBeadId(bead: Bead): string | undefined {
  if (bead.parent) return bead.parent;
  const pc = (bead.dependencies ?? []).find((d) => d.dependency_type === 'parent-child');
  return pc?.id;
}
