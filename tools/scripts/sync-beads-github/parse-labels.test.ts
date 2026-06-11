import { test } from 'node:test';
import assert from 'node:assert/strict';
import { splitLabels } from './parse-labels.ts';

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
