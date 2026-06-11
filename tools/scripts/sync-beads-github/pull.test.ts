import { test } from 'node:test';
import assert from 'node:assert/strict';
import { reconcileIssueToBead } from './pull.ts';
import type { Bead, ReverseBeadsStore, BeadUpdate, CreateBeadInput } from './beads.ts';
import type { GithubIssue } from './github.ts';

const REPO = 'sidereal-io/sidereal';

class FakeStore implements ReverseBeadsStore {
  created: CreateBeadInput[] = [];
  updates: Array<{ id: string; fields: BeadUpdate }> = [];
  closed: string[] = [];
  reopened: string[] = [];
  refs = new Map<string, string>();
  private nextId = 1;
  constructor(public beads: Bead[]) {}
  async list() {
    return this.beads;
  }
  async setExternalRef(id: string, url: string) {
    this.refs.set(id, url);
    const b = this.beads.find((x) => x.id === id);
    if (b) b.external_ref = url;
  }
  async create(input: CreateBeadInput) {
    this.created.push(input);
    const id = `new-${this.nextId++}`;
    this.beads.push({
      id,
      title: input.title,
      status: 'open',
      priority: 2,
      issue_type: input.type ?? 'task',
      external_ref: input.externalRef,
      updated_at: '2026-01-01T00:00:00Z',
    });
    return id;
  }
  async update(id: string, fields: BeadUpdate) {
    this.updates.push({ id, fields });
  }
  async close(id: string) {
    this.closed.push(id);
  }
  async reopen(id: string) {
    this.reopened.push(id);
  }
}

function issue(p: Partial<GithubIssue> & Pick<GithubIssue, 'number'>): GithubIssue {
  return {
    id: p.number,
    title: p.title ?? 't',
    body: p.body ?? null,
    state: p.state ?? 'open',
    updated_at: '2026-06-10T00:00:00Z',
    labels: p.labels ?? [],
    assignees: p.assignees ?? [],
    ...p,
  };
}

function bead(p: Partial<Bead> & Pick<Bead, 'id'>): Bead {
  return {
    title: p.id,
    status: 'open',
    priority: 2,
    issue_type: 'task',
    updated_at: '2026-06-10T00:00:00Z',
    ...p,
  };
}

const deps = (store: FakeStore) => ({ beads: store, repo: REPO, trackLabel: 'beads' });

test('updates a tracked bead from issue fields', async () => {
  const store = new FakeStore([
    bead({ id: 'b-1', external_ref: `https://github.com/${REPO}/issues/10` }),
  ]);
  await reconcileIssueToBead(
    issue({
      number: 10,
      title: 'New title',
      body: 'Desc.\n\n## Design\n\nPlan.',
      labels: ['area/x', 'priority/p1'],
      assignees: ['alice'],
    }),
    deps(store),
  );
  assert.equal(store.updates.length, 1);
  const f = store.updates[0].fields;
  assert.equal(f.title, 'New title');
  assert.equal(f.description, 'Desc.');
  assert.equal(f.design, 'Plan.');
  assert.deepEqual(f.labels, ['area/x']);
  assert.equal(f.priority, 1);
  assert.equal(f.owner, 'alice');
});

test('adopts an untracked issue only if it has the track label', async () => {
  const store = new FakeStore([]);
  await reconcileIssueToBead(issue({ number: 20, title: 'Tracked', labels: ['beads'] }), deps(store));
  assert.equal(store.created.length, 1);
  assert.equal(store.created[0].externalRef, `https://github.com/${REPO}/issues/20`);
  assert.equal(store.updates.length, 1); // create then field update
});

test('ignores an untracked issue without the track label', async () => {
  const store = new FakeStore([]);
  await reconcileIssueToBead(issue({ number: 21, labels: [] }), deps(store));
  assert.equal(store.created.length, 0);
  assert.equal(store.updates.length, 0);
});

test('a closed issue closes the bead', async () => {
  const store = new FakeStore([
    bead({ id: 'b-2', status: 'open', external_ref: `https://github.com/${REPO}/issues/30` }),
  ]);
  await reconcileIssueToBead(issue({ number: 30, state: 'closed' }), deps(store));
  assert.deepEqual(store.closed, ['b-2']);
});

test('a reopened issue reopens a closed bead', async () => {
  const store = new FakeStore([
    bead({ id: 'b-3', status: 'closed', external_ref: `https://github.com/${REPO}/issues/31` }),
  ]);
  await reconcileIssueToBead(issue({ number: 31, state: 'open' }), deps(store));
  assert.deepEqual(store.reopened, ['b-3']);
});

test('status label on an open issue sets status (and un-closes via update)', async () => {
  const store = new FakeStore([
    bead({ id: 'b-4', status: 'closed', external_ref: `https://github.com/${REPO}/issues/32` }),
  ]);
  await reconcileIssueToBead(
    issue({ number: 32, state: 'open', labels: ['status/blocked'] }),
    deps(store),
  );
  assert.equal(store.updates[0].fields.status, 'blocked');
  assert.deepEqual(store.reopened, []); // status update handles un-closing, no separate reopen
});

test('first assignee wins; no assignees clears owner', async () => {
  const store = new FakeStore([
    bead({ id: 'b-5', external_ref: `https://github.com/${REPO}/issues/33` }),
  ]);
  await reconcileIssueToBead(issue({ number: 33, assignees: [] }), deps(store));
  assert.equal(store.updates[0].fields.owner, null);
});
