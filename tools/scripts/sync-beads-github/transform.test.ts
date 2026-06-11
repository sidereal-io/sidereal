import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
  renderBody,
  markerFor,
  extractMarkerId,
  computeManagedLabels,
  reconcileLabels,
  BLOCKED_LABEL,
  githubStateForBead,
  refUrlForNumber,
  parseBody,
  splitLabels,
  issueNumberFromRef,
} from './transform.ts';
import type { Bead } from './beads.ts';

// ===========================================================================
// beads -> GitHub
// ===========================================================================

const base: Bead = {
  id: 'b-5',
  title: 'Task 5',
  status: 'open',
  priority: 2,
  issue_type: 'task',
  updated_at: '2026-06-10T00:00:00Z',
};

test('renders description, design, and blockers in order', () => {
  const bead: Bead = { ...base, description: 'Do X.', design: 'Approach Y.' };
  const out = renderBody(bead, [42, 43]);
  const iDesc = out.indexOf('Do X.');
  const iDesign = out.indexOf('## Design');
  const iBlocked = out.indexOf('### Blocked by');
  assert.ok(iDesc >= 0 && iDesign > iDesc && iBlocked > iDesign);
  assert.ok(out.includes('- #42'));
  assert.ok(out.includes('- #43'));
  assert.ok(out.includes(markerFor('b-5')));
});

test('omits the Design section when design is empty', () => {
  const out = renderBody({ ...base, description: 'Just text.' }, []);
  assert.ok(!out.includes('## Design'));
  assert.ok(!out.includes('### Blocked by'));
});

test('handles an empty description (design-only bead)', () => {
  const out = renderBody({ ...base, design: 'only design' }, []);
  assert.ok(out.startsWith('## Design'));
});

test('extractMarkerId round-trips with markerFor', () => {
  assert.equal(extractMarkerId(markerFor('b-xyz')), 'b-xyz');
  assert.equal(extractMarkerId('no marker here'), null);
  assert.equal(extractMarkerId(null), null);
});

const labelBead: Bead = {
  id: 'b-5',
  title: 'Task 5',
  status: 'in_progress',
  priority: 2,
  issue_type: 'task',
  labels: ['area/sync'],
  updated_at: '2026-06-10T00:00:00Z',
};

test('computeManagedLabels builds slash-prefixed labels', () => {
  const got = computeManagedLabels(labelBead, false).sort();
  assert.deepEqual(got, ['area/sync', 'priority/p2', 'status/in_progress', 'type/task']);
});

test('adds the blocked label when a blocker is open', () => {
  assert.ok(computeManagedLabels(labelBead, true).includes(BLOCKED_LABEL));
});

test('reconcile drops stale managed labels (incl. legacy ::) but keeps human labels', () => {
  const existing = ['type::task', 'priority::medium', 'blocked', 'discussion'];
  const desired = computeManagedLabels(labelBead, false); // not blocked anymore
  const out = reconcileLabels(existing, desired, labelBead.labels ?? []).sort();
  // legacy type::/priority:: and stale "blocked" removed; "discussion" (human) kept
  assert.deepEqual(out, [
    'area/sync',
    'discussion',
    'priority/p2',
    'status/in_progress',
    'type/task',
  ]);
});

const stateBead = (status: string): Bead => ({
  id: 'b-1',
  title: 't',
  status,
  priority: 2,
  issue_type: 'task',
  updated_at: '2026-06-10T00:00:00Z',
});

test('closed bead -> closed GitHub state', () => {
  assert.equal(githubStateForBead(stateBead('closed')), 'closed');
});

test('open bead -> open GitHub state', () => {
  assert.equal(githubStateForBead(stateBead('open')), 'open');
});

test('in_progress bead -> open GitHub state', () => {
  assert.equal(githubStateForBead(stateBead('in_progress')), 'open');
});

test('builds an issue URL from a number', () => {
  assert.equal(
    refUrlForNumber('sidereal-io/sidereal', 5),
    'https://github.com/sidereal-io/sidereal/issues/5',
  );
});

// ===========================================================================
// GitHub -> beads
// ===========================================================================

const bead: Bead = {
  id: 'b-1',
  title: 't',
  status: 'open',
  priority: 2,
  issue_type: 'task',
  updated_at: '2026-06-10T00:00:00Z',
};

test('round-trips description + design from a rendered body', () => {
  const body = renderBody({ ...bead, description: 'Do the thing.', design: 'Approach: Y.' }, [42]);
  const out = parseBody(body);
  assert.equal(out.description, 'Do the thing.');
  assert.equal(out.design, 'Approach: Y.');
});

test('description only (no design section) leaves design undefined', () => {
  const body = renderBody({ ...bead, description: 'Just text.' }, []);
  const out = parseBody(body);
  assert.equal(out.description, 'Just text.');
  assert.equal(out.design, undefined);
});

test('strips the Blocked by section and the marker', () => {
  const body = renderBody({ ...bead, description: 'D', design: 'G' }, [1, 2]);
  const out = parseBody(body);
  assert.ok(!out.description.includes('Blocked by'));
  assert.ok(!(out.design ?? '').includes('Blocked by'));
  assert.ok(!out.description.includes('beads-sync'));
});

test('fallback: mangled Design heading -> whole text is description, design undefined', () => {
  const out = parseBody('Some text.\n\n##Design\nnot a real heading\n');
  assert.ok(out.description.includes('Some text.'));
  assert.ok(out.description.includes('##Design'));
  assert.equal(out.design, undefined);
});

test('handles null/empty body', () => {
  assert.deepEqual(parseBody(null), { description: '' });
  assert.deepEqual(parseBody(''), { description: '' });
});

test('splits free-form from structured and ignores indicators', () => {
  const out = splitLabels(
    ['area/sync', 'type/bug', 'priority/p1', 'status/in_progress', 'beads', 'blocked'],
    'beads',
  );
  assert.deepEqual(out.freeform, ['area/sync']);
  assert.equal(out.issueType, 'bug');
  assert.equal(out.priority, 1);
  assert.equal(out.status, 'in_progress');
  assert.equal(out.warnings.length, 0);
});

test('invalid structured values are dropped with a warning', () => {
  const out = splitLabels(['type/chore', 'priority/p9', 'status/zzz'], 'beads');
  assert.equal(out.issueType, 'chore'); // chore IS valid
  assert.equal(out.priority, undefined);
  assert.equal(out.status, undefined);
  assert.equal(out.warnings.length, 2); // priority/p9 + status/zzz
});

test('no labels -> empty freeform, all fields undefined', () => {
  const out = splitLabels([], 'beads');
  assert.deepEqual(out.freeform, []);
  assert.equal(out.issueType, undefined);
});

test('parses a full issue URL', () => {
  assert.equal(issueNumberFromRef('https://github.com/sidereal-io/sidereal/issues/111'), 111);
});

test('parses a trailing-slash URL', () => {
  assert.equal(issueNumberFromRef('https://github.com/x/y/issues/42/'), 42);
});

test('parses a bare number', () => {
  assert.equal(issueNumberFromRef('117'), 117);
});

test('returns null for undefined or junk', () => {
  assert.equal(issueNumberFromRef(undefined), null);
  assert.equal(issueNumberFromRef('jira-ABC'), null);
});
