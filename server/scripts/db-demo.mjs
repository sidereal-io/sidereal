#!/usr/bin/env node
// Retained-state demonstration, restricted to this checkout's fixture.
import assert from 'node:assert/strict';
import { execFile, spawn } from 'node:child_process';
import { once } from 'node:events';
import { closeSync, openSync } from 'node:fs';
import { mkdir, mkdtemp, rm, writeFile } from 'node:fs/promises';
import { createServer } from 'node:net';
import { resolve } from 'node:path';
import { setTimeout as sleep } from 'node:timers/promises';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';

const root = fileURLToPath(new URL('../../', import.meta.url));
const execute = promisify(execFile);
const interrupted = new AbortController();
for (const signal of ['SIGINT', 'SIGTERM']) {
  process.on(signal, () => interrupted.abort());
}

async function command(program, args) {
  const { stdout } = await execute(program, args, {
    cwd: root,
    signal: interrupted.signal,
    maxBuffer: 4 * 1024 * 1024,
  });
  return stdout.trim();
}

const fixture = (action) => command('sh', ['server/scripts/db-fixture.sh', action]);

async function request(port, route, timeout = 2000) {
  const response = await fetch(`http://127.0.0.1:${port}/${route}`, {
    signal: AbortSignal.any([interrupted.signal, AbortSignal.timeout(timeout)]),
  });
  return { status: response.status, body: await response.json() };
}

async function waitReady(port) {
  const deadline = performance.now() + 30_000;
  while (performance.now() < deadline) {
    try {
      const response = await request(port, 'readyz');
      if (response.status === 200 && response.body.status === 'ready') return;
    } catch {
      interrupted.signal.throwIfAborted();
    }
    await sleep(100, undefined, { signal: interrupted.signal });
  }
  throw new Error('demo readiness failed within 30 seconds');
}

async function main() {
  await mkdir(resolve(root, '.workspace'), { recursive: true });
  await fixture('up');
  await command('cargo', ['build', '-p', 'sidereal-server', '--manifest-path', 'server/Cargo.toml']);
  const metadata = JSON.parse(await command('cargo', [
    'metadata', '--manifest-path', 'server/Cargo.toml', '--no-deps', '--format-version', '1',
  ]));
  const binary = resolve(metadata.target_directory, 'debug/sidereal-server');
  const selected = createServer();
  selected.listen(0, '127.0.0.1');
  await once(selected, 'listening');
  const port = selected.address().port;
  await new Promise((resolve, reject) => selected.close((error) => error ? reject(error) : resolve()));
  const env = { ...process.env, DATABASE_URL: await fixture('demo-url'), PORT: String(port) };
  const scratch = await mkdtemp(resolve(root, '.workspace/db-demo-'));
  const log = openSync(resolve(scratch, 'server.log'), 'w');
  let server;
  let exited;
  let passed = false;

  async function start() {
    interrupted.signal.throwIfAborted();
    server = spawn(binary, [], { cwd: root, env, stdio: ['ignore', log, log] });
    exited = new Promise((resolve) => {
      server.once('exit', resolve);
      server.once('error', resolve);
    });
    await once(server, 'spawn');
    await waitReady(port);
  }

  async function stop() {
    if (!server || server.exitCode !== null || server.signalCode !== null) return;
    server.kill('SIGTERM');
    const deadline = setTimeout(() => server.kill('SIGKILL'), 5000);
    try {
      await exited;
    } finally {
      clearTimeout(deadline);
    }
  }

  try {
    await start();
    const before = await fixture('demo-records');
    await writeFile(resolve(scratch, 'initial.json'), before);
    await stop();
    await start();
    assert.equal(await fixture('demo-records'), before, 'migration records changed after server restart');

    // Prove database outage changes readiness while liveness stays healthy.
    await fixture('stop');
    assert.deepEqual(await request(port, 'healthz'), { status: 200, body: { status: 'ok' } });
    const started = performance.now();
    assert.deepEqual(await request(port, 'readyz', 2250), { status: 503, body: { status: 'not_ready' } });
    assert(performance.now() - started < 2250, 'outage readiness exceeded deadline');
    await fixture('up');
    const recovered = performance.now();
    await waitReady(port);
    assert(performance.now() - recovered < 5000, 'readiness recovery exceeded five seconds');
    assert.equal(await fixture('demo-records'), before, 'migration records changed after PostgreSQL restart');

    // Also prove down/up retains the named volume and the complete history.
    await stop();
    await fixture('down');
    await fixture('up');
    await start();
    assert.equal(await fixture('demo-records'), before, 'fixture stop/start changed migration records');
    console.log('Database demo passed: identical migration records after server and PostgreSQL restarts.');
    passed = true;
  } finally {
    await stop();
    closeSync(log);
    if (passed) await rm(scratch, { recursive: true });
    else console.error(`Demo logs and initial migration records: ${scratch}`);
  }
}

try {
  await main();
} catch (error) {
  if (interrupted.signal.aborted) process.exitCode = 130;
  else {
    console.error(error);
    process.exitCode = 1;
  }
}
