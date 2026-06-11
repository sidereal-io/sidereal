import { test } from 'node:test';
import assert from 'node:assert/strict';
import { syncBeadsToGithub } from './push.ts';
import type { Bead, BeadsStore, CreateBeadInput } from './beads.ts';
import type { GithubClient, GithubIssue, CreateIssueInput, UpdateIssueInput } from './github.ts';

class FakeGithub implements GithubClient {
  issues = new Map<number, GithubIssue>();
  subIssues = new Map<number, number[]>(); // parentNum -> child numbers
  ensured: string[] = [];
  setLabelsCalls: Array<{ num: number; labels: string[] }> = [];
  assigneeCalls: Array<{ num: number; logins: string[] }> = [];
  private nextNum: number;
  private nextId = 1000;

  constructor(seed: GithubIssue[] = []) {
    for (const i of seed) this.issues.set(i.number, i);
    this.nextNum = Math.max(0, ...seed.map((i) => i.number)) + 1;
  }
  async listOpenIssues() {
    return [...this.issues.values()].filter((i) => i.state === 'open');
  }
  async getIssue(num: number) {
    return this.issues.get(num) ?? null;
  }
  async createIssue(input: CreateIssueInput) {
    const issue: GithubIssue = {
      number: this.nextNum++,
      id: this.nextId++,
      title: input.title,
      body: input.body ?? null,
      state: 'open',
      updated_at: '2026-01-01T00:00:00Z',
      labels: [],
      assignees: [],
    };
    this.issues.set(issue.number, issue);
    return issue;
  }
  async updateIssue(num: number, input: UpdateIssueInput) {
    const i = this.issues.get(num)!;
    if (input.title !== undefined) i.title = input.title;
    if (input.body !== undefined) i.body = input.body;
    if (input.state !== undefined) i.state = input.state;
  }
  async ensureLabelsExist(labels: string[]) {
    this.ensured.push(...labels);
  }
  async setLabels(num: number, labels: string[]) {
    this.issues.get(num)!.labels = labels;
    this.setLabelsCalls.push({ num, labels });
  }
  async setAssignees(num: number, logins: string[]) {
    this.issues.get(num)!.assignees = logins;
    this.assigneeCalls.push({ num, logins });
  }
  async listSubIssues(parentNum: number) {
    return this.subIssues.get(parentNum) ?? [];
  }
  async addSubIssue(parentNum: number, childId: number) {
    const childNum = [...this.issues.values()].find((i) => i.id === childId)!.number;
    const arr = this.subIssues.get(parentNum) ?? [];
    arr.push(childNum);
    this.subIssues.set(parentNum, arr);
  }
}

class FakeStore implements BeadsStore {
  created: CreateBeadInput[] = [];
  refs = new Map<string, string>();
  private nextId = 1;
  constructor(public beads: Bead[]) {}
  async list() {
    return this.beads;
  }
  async setExternalRef(beadId: string, url: string) {
    this.refs.set(beadId, url);
    const b = this.beads.find((x) => x.id === beadId);
    if (b) b.external_ref = url;
  }
  async create(input: CreateBeadInput) {
    this.created.push(input);
    const id = `new-${this.nextId++}`;
    this.beads.push({
      id,
      title: input.title,
      description: input.description,
      status: 'open',
      priority: 2,
      issue_type: input.type ?? 'task',
      external_ref: input.externalRef,
      updated_at: '2026-01-01T00:00:00Z',
    });
    return id;
  }
}

const REPO = 'sidereal-io/sidereal';
const NOW = '2026-06-10T00:00:00Z';

function bead(partial: Partial<Bead> & Pick<Bead, 'id'>): Bead {
  return {
    title: partial.id,
    status: 'open',
    priority: 2,
    issue_type: 'task',
    updated_at: NOW,
    ...partial,
  };
}

test('creates a GitHub issue for an unmapped bead and records external_ref', async () => {
  const store = new FakeStore([bead({ id: 'b-1', description: 'hello', design: 'plan' })]);
  const gh = new FakeGithub();
  await syncBeadsToGithub({ beads: store, github: gh, repo: REPO, dryRun: false, trackLabel: 'beads' });

  assert.equal(gh.issues.size, 1);
  const issue = [...gh.issues.values()][0];
  assert.equal(issue.title, 'b-1');
  assert.ok(issue.body!.includes('hello'));
  assert.ok(issue.body!.includes('## Design'));
  assert.equal(store.refs.get('b-1'), `https://github.com/${REPO}/issues/${issue.number}`);
  assert.ok(issue.labels.includes('type/task'));
});

test('renders blockers and adds the blocked label using mapped issue numbers', async () => {
  const blocker = bead({ id: 'b-block', status: 'open', external_ref: `https://github.com/${REPO}/issues/50` });
  const blocked = bead({
    id: 'b-main',
    external_ref: `https://github.com/${REPO}/issues/51`,
    dependencies: [{ id: 'b-block', dependency_type: 'blocks', status: 'open' }],
  });
  const store = new FakeStore([blocker, blocked]);
  const gh = new FakeGithub([
    { number: 50, id: 1, title: 'b-block', body: null, state: 'open', updated_at: NOW, labels: [], assignees: [] },
    { number: 51, id: 2, title: 'b-main', body: null, state: 'open', updated_at: NOW, labels: [], assignees: [] },
  ]);
  await syncBeadsToGithub({ beads: store, github: gh, repo: REPO, dryRun: false, trackLabel: 'beads' });

  const main = gh.issues.get(51)!;
  assert.ok(main.body!.includes('### Blocked by'));
  assert.ok(main.body!.includes('- #50'));
  assert.ok(main.labels.includes('blocked'));
});

test('links a child bead as a native sub-issue of its parent', async () => {
  const parent = bead({ id: 'b-parent', issue_type: 'epic', external_ref: `https://github.com/${REPO}/issues/60` });
  const child = bead({ id: 'b-child', parent: 'b-parent', external_ref: `https://github.com/${REPO}/issues/61` });
  const store = new FakeStore([parent, child]);
  const gh = new FakeGithub([
    { number: 60, id: 10, title: 'b-parent', body: null, state: 'open', updated_at: NOW, labels: [], assignees: [] },
    { number: 61, id: 11, title: 'b-child', body: null, state: 'open', updated_at: NOW, labels: [], assignees: [] },
  ]);
  await syncBeadsToGithub({ beads: store, github: gh, repo: REPO, dryRun: false, trackLabel: 'beads' });

  assert.deepEqual(await gh.listSubIssues(60), [61]);
});

test('adopts a human-created GitHub issue only when it has the track label', async () => {
  const store = new FakeStore([]);
  const gh = new FakeGithub([
    { number: 70, id: 20, title: 'Human bug', body: 'broke', state: 'open', updated_at: NOW, labels: ['beads'], assignees: [] },
    { number: 72, id: 22, title: 'Untracked', body: 'x', state: 'open', updated_at: NOW, labels: [], assignees: [] },
  ]);
  await syncBeadsToGithub({ beads: store, github: gh, repo: REPO, dryRun: false, trackLabel: 'beads' });
  assert.equal(store.created.length, 1);
  assert.equal(store.created[0].title, 'Human bug');
});

test('does not re-adopt an issue that already carries our marker', async () => {
  const store = new FakeStore([]);
  const gh = new FakeGithub([
    {
      number: 71,
      id: 21,
      title: 'Mirrored',
      body: 'text\n\n<!-- managed by beads-sync · b-x · do not edit -->',
      state: 'open',
      updated_at: NOW,
      labels: [],
      assignees: [],
    },
  ]);
  await syncBeadsToGithub({ beads: store, github: gh, repo: REPO, dryRun: false, trackLabel: 'beads' });
  assert.equal(store.created.length, 0);
});

test('dry run performs no writes', async () => {
  const store = new FakeStore([bead({ id: 'b-1' })]);
  const gh = new FakeGithub();
  await syncBeadsToGithub({ beads: store, github: gh, repo: REPO, dryRun: true, trackLabel: 'beads' });
  assert.equal(gh.issues.size, 0);
  assert.equal(store.refs.size, 0);
});

test('closed bead closes its GitHub issue (one-directional mirror)', async () => {
  const store = new FakeStore([
    bead({ id: 'b-done', status: 'closed', external_ref: `https://github.com/${REPO}/issues/80` }),
  ]);
  const gh = new FakeGithub([
    { number: 80, id: 1, title: 'b-done', body: null, state: 'open', updated_at: NOW, labels: [], assignees: [] },
  ]);
  await syncBeadsToGithub({ beads: store, github: gh, repo: REPO, dryRun: false, trackLabel: 'beads' });
  assert.equal(gh.issues.get(80)!.state, 'closed');
});

test('reopens a GitHub issue a human closed while the bead is open', async () => {
  const store = new FakeStore([
    bead({ id: 'b-open', status: 'open', external_ref: `https://github.com/${REPO}/issues/81` }),
  ]);
  const gh = new FakeGithub();
  // seed a CLOSED issue (FakeGithub.listOpenIssues won't return it, but getIssue will)
  gh.issues.set(81, { number: 81, id: 2, title: 'b-open', body: null, state: 'closed', updated_at: NOW, labels: [], assignees: [] });
  await syncBeadsToGithub({ beads: store, github: gh, repo: REPO, dryRun: false, trackLabel: 'beads' });
  assert.equal(gh.issues.get(81)!.state, 'open');
});

test('skips gate beads and ignores gate blockers (no issue, no blocked label)', async () => {
  const gate = bead({ id: 'g-1', issue_type: 'gate', status: 'open' });
  const work = bead({
    id: 'b-work',
    external_ref: `https://github.com/${REPO}/issues/82`,
    dependencies: [{ id: 'g-1', dependency_type: 'blocks', status: 'open' }],
  });
  const store = new FakeStore([gate, work]);
  const gh = new FakeGithub([
    { number: 82, id: 3, title: 'b-work', body: null, state: 'open', updated_at: NOW, labels: [], assignees: [] },
  ]);
  await syncBeadsToGithub({ beads: store, github: gh, repo: REPO, dryRun: false, trackLabel: 'beads' });

  // gate produced no GitHub issue
  assert.equal([...gh.issues.values()].some((i) => i.title === 'g-1'), false);
  // work issue is NOT marked blocked and has no "Blocked by" section (gate excluded)
  const work82 = gh.issues.get(82)!;
  assert.equal(work82.labels.includes('blocked'), false);
  assert.equal(work82.body!.includes('### Blocked by'), false);
});

test('does not create a GitHub issue for a closed bead that was never synced', async () => {
  const store = new FakeStore([bead({ id: 'b-old', status: 'closed' })]); // no external_ref
  const gh = new FakeGithub();
  await syncBeadsToGithub({ beads: store, github: gh, repo: REPO, dryRun: false, trackLabel: 'beads' });
  assert.equal(gh.issues.size, 0);
  assert.equal(store.refs.size, 0);
});

test('forward sync labels every issue with the track label', async () => {
  const store = new FakeStore([bead({ id: 'b-1' })]);
  const gh = new FakeGithub();
  await syncBeadsToGithub({ beads: store, github: gh, repo: REPO, dryRun: false, trackLabel: 'beads' });
  const issue = [...gh.issues.values()][0];
  assert.ok(issue.labels.includes('beads'));
});

test('recreates the issue when a bead references a deleted (gone) GitHub issue', async () => {
  // bead points at #154, which no longer exists (getIssue returns null on 404/410)
  const store = new FakeStore([
    bead({ id: 'b-1', external_ref: `https://github.com/${REPO}/issues/154` }),
  ]);
  const gh = new FakeGithub(); // #154 is absent
  await syncBeadsToGithub({ beads: store, github: gh, repo: REPO, dryRun: false, trackLabel: 'beads' });

  assert.equal(gh.issues.size, 1);
  const recreated = [...gh.issues.values()][0];
  assert.notEqual(recreated.number, 154); // a brand-new issue, not the deleted one
  assert.equal(
    store.refs.get('b-1'),
    `https://github.com/${REPO}/issues/${recreated.number}`,
  );
});
