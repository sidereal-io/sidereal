import { test } from 'node:test';
import assert from 'node:assert/strict';
import { parseBody } from './parse-body.ts';
import { renderBody } from './body.ts';
import type { Bead } from './bead.ts';

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
