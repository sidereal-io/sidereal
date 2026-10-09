import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import vm from 'node:vm';

// Exercise the deployed GitHub Script, including its rendered PR comment.
const workflow = readFileSync(new URL('../../.github/workflows/docker-build-test.yml', import.meta.url), 'utf8');
const script = workflow.split('          script: |\n')[1]
  .split('\n').map(line => line.slice(12)).join('\n');

async function comment(outcome, report, otherOutcome = 'success') {
  let body;
  const rendered = script.replace(/\$\{\{\s*([^}]+?)\s*\}\}/g, (_, expression) => {
    if (expression === 'steps.security-report.outcome') return outcome;
    if (expression.endsWith('.outcome')) return otherOutcome;
    return 'fixture';
  });
  await vm.runInNewContext(`(async () => {${rendered}})()`, {
    require: () => ({ readFileSync: () => {
      if (report === undefined) throw new Error('Report missing');
      return report;
    } }),
    context: { issue: { number: 1 }, repo: { owner: 'fixture', repo: 'fixture' } },
    github: { rest: { issues: { createComment: async data => { body = data.body; } } } },
  });
  return body;
}

test('successful reports show findings or an empty result', async () => {
  assert.match(await comment('success', ''), /No vulnerabilities found/);
  assert.match(await comment('success', 'CRITICAL HIGH MEDIUM'), /1 Critical/);
  assert.match(await comment('success', 'CRITICAL HIGH MEDIUM'), /critical vulnerabilities found/);
});

for (const outcome of ['failure', 'skipped', 'cancelled', '']) {
  for (const report of [undefined, '', 'CRITICAL stale report']) {
    test(`${outcome || 'missing'} outcome cannot trust ${report === undefined ? 'missing' : 'stale'} report`, async () => {
      const body = await comment(outcome, report);
      assert.doesNotMatch(body, /All checks passed|No vulnerabilities found|critical vulnerabilities found/);
      assert.match(body, /report not available/);
    });
  }
}

test('a successful step without a report is not a successful security check', async () => {
  const body = await comment('success', undefined);
  assert.doesNotMatch(body, /All checks passed|No vulnerabilities found/);
  assert.match(body, /report not available/);
});

test('findings do not hide other build failures', async () => {
  assert.match(await comment('success', 'CRITICAL', 'failure'), /step\(s\) failed/);
});
