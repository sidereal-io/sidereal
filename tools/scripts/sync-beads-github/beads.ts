import { execFile } from 'node:child_process';
import { promisify } from 'node:util';

const exec = promisify(execFile);

// --- domain model ----------------------------------------------------------

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

// --- bd CLI store ----------------------------------------------------------

export interface CreateBeadInput {
  title: string;
  description?: string;
  type?: string;
  externalRef: string;
}

// NOTE: deliberately no close()/reopen() — the sync agent must never change a
// bead's open/closed state (see Task 11). Closing happens via human `bd close`
// or the PR-merge gate workflow (Task 12), never through this store.
export interface BeadsStore {
  list(): Promise<Bead[]>;
  setExternalRef(beadId: string, url: string): Promise<void>;
  create(input: CreateBeadInput): Promise<string>; // returns new bead id
}

export interface BeadUpdate {
  title?: string;
  description?: string;
  design?: string;
  owner?: string | null; // null clears the assignee
  priority?: number;
  issueType?: string;
  status?: string;
  labels?: string[]; // full free-form label set (replaces existing)
}

/**
 * Reverse-sync store: extends the forward store with the mutations a human's
 * GitHub action maps to. The forward sync depends only on `BeadsStore`, so it
 * cannot close/reopen beads (autonomous-close guarantee). Only the human-event
 * reverse path uses these.
 */
export interface ReverseBeadsStore extends BeadsStore {
  update(beadId: string, fields: BeadUpdate): Promise<void>;
  close(beadId: string): Promise<void>;
  reopen(beadId: string): Promise<void>;
}

async function bd(args: string[]): Promise<string> {
  const { stdout } = await exec('bd', args, {
    env: { ...process.env, BD_NON_INTERACTIVE: '1' },
    maxBuffer: 64 * 1024 * 1024,
  });
  return stdout;
}

export class BdCliStore implements ReverseBeadsStore {
  async list(): Promise<Bead[]> {
    // --all: include closed beads (needed to mirror closed state to GitHub).
    // --include-gates: gates are hidden by default; the sync needs them to
    // identify gate-type blockers (and to exclude gate beads from mirroring).
    return parseBeads(await bd(['list', '--all', '--include-gates', '--json']));
  }

  async setExternalRef(beadId: string, url: string): Promise<void> {
    await bd(['update', beadId, '--external-ref', url]);
  }

  async create(input: CreateBeadInput): Promise<string> {
    const args = ['create', '--title', input.title, '--type', input.type ?? 'task', '--json'];
    if (input.description) args.push('--description', input.description);
    const out = await bd(args);
    const parsed = JSON.parse(out) as { id?: string } | Array<{ id?: string }>;
    const id = Array.isArray(parsed) ? parsed[0]?.id : parsed.id;
    if (!id) throw new Error(`bd create did not return an id: ${out}`);
    await this.setExternalRef(id, input.externalRef);
    return id;
  }

  async update(beadId: string, fields: BeadUpdate): Promise<void> {
    const args = ['update', beadId];
    if (fields.title !== undefined) args.push('--title', fields.title);
    if (fields.description !== undefined) args.push('--description', fields.description);
    if (fields.design !== undefined) args.push('--design', fields.design);
    if (fields.issueType !== undefined) args.push('--type', fields.issueType);
    if (fields.priority !== undefined) args.push('--priority', String(fields.priority));
    if (fields.status !== undefined) args.push('--status', fields.status);
    if (fields.owner !== undefined) args.push('--assignee', fields.owner ?? '');
    if (fields.labels !== undefined) args.push('--set-labels', fields.labels.join(','));
    if (args.length === 2) return; // nothing to change
    await bd(args);
  }

  async close(beadId: string): Promise<void> {
    await bd(['close', beadId, '--reason', 'closed on GitHub']);
  }

  async reopen(beadId: string): Promise<void> {
    await bd(['update', beadId, '--status', 'open']);
  }
}
