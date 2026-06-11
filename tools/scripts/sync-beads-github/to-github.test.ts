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
} from './to-github.ts';
import type { Bead } from './bead.ts';

// --- body ------------------------------------------------------------------

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

// --- labels ----------------------------------------------------------------

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

// --- state -----------------------------------------------------------------

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

// --- external ref ----------------------------------------------------------

test('builds an issue URL from a number', () => {
  assert.equal(
    refUrlForNumber('sidereal-io/sidereal', 5),
    'https://github.com/sidereal-io/sidereal/issues/5',
  );
});
