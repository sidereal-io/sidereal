import assert from 'node:assert/strict';
import { readFileSync, mkdirSync, mkdtempSync, writeFileSync, rmSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { join } from 'node:path';
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

const buildWorkflow = readFileSync(new URL('../../.github/workflows/docker-build-push.yml', import.meta.url), 'utf8');
const selection = buildWorkflow.split('      - name: Select immutable scan image\n')[1]
  .split('        run: |\n')[1].split('\n      - name:')[0]
  .split('\n').map(line => line.slice(10)).join('\n');
const scratch = new URL('../../.workspace/', import.meta.url);
mkdirSync(scratch, { recursive: true });
const digest = 'a'.repeat(64);

for (const [name, entries, successful] of [
  ['valid digest', [digest], true],
  ['no digest', [], false],
  ['multiple digests', [digest, 'b'.repeat(64)], false],
  ['malformed digest', ['sha256-invalid'], false],
  ['non-hex digest', ['g'.repeat(64)], false],
  ['hidden extra file', [digest, '.extra'], false],
]) {
  test(`scan target: ${name}`, () => {
    const directory = mkdtempSync(new URL('scan-test-', scratch).pathname);
    try {
      const digestDir = join(directory, 'digests');
      mkdirSync(digestDir);
      for (const entry of entries) writeFileSync(join(digestDir, entry), '');
      const output = join(directory, 'output');
      const result = spawnSync('bash', ['-c', selection], {
        env: { ...process.env, DIGEST_DIR: digestDir, GITHUB_OUTPUT: output,
          REGISTRY: 'ghcr.io', IMAGE_NAME: 'fixture/sidereal' },
        encoding: 'utf8',
      });
      if (successful) {
        assert.equal(result.status, 0, result.stderr);
        assert.equal(readFileSync(output, 'utf8'), `image-ref=ghcr.io/fixture/sidereal@sha256:${digest}\n`);
      } else {
        assert.notEqual(result.status, 0, 'Unsafe scan target accepted');
      }
    } finally {
      rmSync(directory, { recursive: true, force: true });
    }
  });
}
