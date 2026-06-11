import { test } from 'node:test';
import assert from 'node:assert/strict';
import { parseBody, splitLabels, issueNumberFromRef } from './from-github.ts';
import { renderBody } from './to-github.ts';
import type { Bead } from './bead.ts';

// --- body ------------------------------------------------------------------

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

// --- labels ----------------------------------------------------------------

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

// --- external ref ----------------------------------------------------------

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
