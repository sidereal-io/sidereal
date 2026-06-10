import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { parseBeads, type Bead } from './bead.ts';

const exec = promisify(execFile);

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

async function bd(args: string[]): Promise<string> {
  const { stdout } = await exec('bd', args, {
    env: { ...process.env, BD_NON_INTERACTIVE: '1' },
    maxBuffer: 64 * 1024 * 1024,
  });
  return stdout;
}

export class BdCliStore implements BeadsStore {
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
}
