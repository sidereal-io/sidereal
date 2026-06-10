import { test } from 'node:test';
import assert from 'node:assert/strict';
import { renderBody, markerFor, extractMarkerId } from './body.ts';
import type { Bead } from './bead.ts';

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
