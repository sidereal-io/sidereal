import { test } from 'node:test';
import assert from 'node:assert/strict';
import { computeManagedLabels, reconcileLabels, BLOCKED_LABEL } from './labels.ts';
import type { Bead } from './bead.ts';

const bead: Bead = {
  id: 'b-5',
  title: 'Task 5',
  status: 'in_progress',
  priority: 2,
  issue_type: 'task',
  labels: ['area/sync'],
  updated_at: '2026-06-10T00:00:00Z',
};

test('computeManagedLabels builds slash-prefixed labels', () => {
  const got = computeManagedLabels(bead, false).sort();
  assert.deepEqual(got, ['area/sync', 'priority/p2', 'status/in_progress', 'type/task']);
});

test('adds the blocked label when a blocker is open', () => {
  assert.ok(computeManagedLabels(bead, true).includes(BLOCKED_LABEL));
});

test('reconcile drops stale managed labels (incl. legacy ::) but keeps human labels', () => {
  const existing = ['type::task', 'priority::medium', 'blocked', 'discussion'];
  const desired = computeManagedLabels(bead, false); // not blocked anymore
  const out = reconcileLabels(existing, desired, bead.labels ?? []).sort();
  // legacy type::/priority:: and stale "blocked" removed; "discussion" (human) kept
  assert.deepEqual(out, [
    'area/sync',
    'discussion',
    'priority/p2',
    'status/in_progress',
    'type/task',
  ]);
});
