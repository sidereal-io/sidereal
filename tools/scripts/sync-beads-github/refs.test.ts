import { test } from 'node:test';
import assert from 'node:assert/strict';
import { issueNumberFromRef, refUrlForNumber } from './refs.ts';

test('parses a full issue URL', () => {
  assert.equal(
    issueNumberFromRef('https://github.com/sidereal-io/sidereal/issues/111'),
    111,
  );
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

test('builds an issue URL from a number', () => {
  assert.equal(
    refUrlForNumber('sidereal-io/sidereal', 5),
    'https://github.com/sidereal-io/sidereal/issues/5',
  );
});
