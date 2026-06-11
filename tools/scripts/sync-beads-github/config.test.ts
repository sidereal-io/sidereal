import { test } from 'node:test';
import assert from 'node:assert/strict';
import { trackLabel, botLogin } from './config.ts';

test('trackLabel defaults to "beads"', () => {
  delete process.env.BEADS_TRACK_LABEL;
  assert.equal(trackLabel(), 'beads');
});

test('trackLabel honors BEADS_TRACK_LABEL', () => {
  process.env.BEADS_TRACK_LABEL = 'track';
  assert.equal(trackLabel(), 'track');
  delete process.env.BEADS_TRACK_LABEL;
});

test('botLogin defaults to github-actions[bot]', () => {
  delete process.env.BEADS_SYNC_BOT;
  assert.equal(botLogin(), 'github-actions[bot]');
});
