import { test } from 'node:test';
import assert from 'node:assert/strict';
import { parseBeads, blockerBeadIds, parentBeadId } from './beads.ts';

const sample = JSON.stringify([
  {
    id: 'b-5',
    title: 'Task 5',
    design: 'do the thing',
    status: 'open',
    priority: 2,
    issue_type: 'task',
    owner: 'someone@example.com',
    updated_at: '2026-06-10T00:08:28Z',
    parent: 'b-parent',
    dependencies: [
      { id: 'b-parent', dependency_type: 'parent-child', status: 'open' },
      { id: 'b-1', dependency_type: 'blocks', status: 'closed' },
      { id: 'b-4', dependency_type: 'blocks', status: 'open' },
      { id: 'b-9', dependency_type: 'related', status: 'open' },
    ],
  },
]);

test('parseBeads returns the array', () => {
  const beads = parseBeads(sample);
  assert.equal(beads.length, 1);
  assert.equal(beads[0].id, 'b-5');
});

test('blockerBeadIds returns only "blocks" dependencies', () => {
  const beads = parseBeads(sample);
  assert.deepEqual(blockerBeadIds(beads[0]), ['b-1', 'b-4']);
});

test('parentBeadId prefers the parent field', () => {
  const beads = parseBeads(sample);
  assert.equal(parentBeadId(beads[0]), 'b-parent');
});

test('parentBeadId falls back to a parent-child dependency', () => {
  const bead = { ...parseBeads(sample)[0], parent: undefined };
  assert.equal(parentBeadId(bead), 'b-parent');
});

test('parseBeads throws on non-array input', () => {
  assert.throws(() => parseBeads('{}'));
});
