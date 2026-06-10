import { test } from 'node:test';
import assert from 'node:assert/strict';
import { githubStateForBead } from './state.ts';
import type { Bead } from './bead.ts';

const bead = (status: string): Bead => ({
  id: 'b-1',
  title: 't',
  status,
  priority: 2,
  issue_type: 'task',
  updated_at: '2026-06-10T00:00:00Z',
});

test('closed bead -> closed GitHub state', () => {
  assert.equal(githubStateForBead(bead('closed')), 'closed');
});

test('open bead -> open GitHub state', () => {
  assert.equal(githubStateForBead(bead('open')), 'open');
});

test('in_progress bead -> open GitHub state', () => {
  assert.equal(githubStateForBead(bead('in_progress')), 'open');
});
