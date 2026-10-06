import { it } from 'node:test';
import assert from 'node:assert/strict';
import type { TestContext } from 'node:test';
import type { PlateSolvingJob } from '@shared/types';

delete process.env.DATABASE_URL;
process.env.SQLITE_DB_PATH = ':memory:';
const { AstrometryService, setWsManager } = await import('./astrometry');
const { storage } = await import('./storage');
const { configService } = await import('./config');

function fixture(t: TestContext) {
  const job: PlateSolvingJob = {
    id: 14, imageId: null, astrometrySubmissionId: '16141301',
    astrometryJobId: null, status: 'processing', result: null,
    submittedAt: new Date(), completedAt: null,
  };
  const writes: Partial<PlateSolvingJob>[] = [];
  t.mock.method(storage, 'getPlateSolvingJob', async () => job);
  t.mock.method(storage, 'updatePlateSolvingJob', async (_id, patch) => {
    writes.push(structuredClone(patch));
    Object.assign(job, patch);
    return job;
  });
  t.mock.method(configService, 'getAstrometryConfig', async () => ({
    apiKey: 'test-key', enabled: true, autoEnabled: false,
    checkInterval: 30, pollInterval: 5, maxConcurrent: 3, autoResubmit: false,
  }));
  const delays: number[] = [];
  t.mock.method(globalThis, 'setTimeout', (callback, ms) => {
    delays.push(Number(ms));
    queueMicrotask(callback);
    return 0;
  });
  t.mock.method(console, 'error', () => {});
  t.mock.method(console, 'log', () => {});
  t.mock.method(console, 'warn', () => {});
  const service = new AstrometryService(false);
  return { job, writes, delays, service };
}

it('keeps null placeholders processing with a None timestamp', async (t) => {
  const h = fixture(t);
  const calls: string[] = [];
  t.mock.method(globalThis, 'fetch', async (url) => {
    assert.equal(String(url), 'https://nova.astrometry.net/api/submissions/16141301');
    calls.push(String(url));
    return Response.json({ jobs: [null], processing_finished: 'None' });
  });
  assert.equal((await h.service.checkJobStatus(14)).status, 'processing');
  assert.deepEqual(h.writes, []);
  assert.equal(calls.length, 1);
});

it('direct polling waits past null placeholders', async (t) => {
  const h = fixture(t);
  let submissionChecks = 0;
  const result = {
    calibration: { ra: 1, dec: 2, pixscale: 3, radius: 4, orientation: 5 },
    annotations: [], machineTags: [],
  };
  t.mock.method(h.service, 'fetchCompleteResult', async () => result);
  t.mock.method(globalThis, 'fetch', async (url) => {
    if (String(url) === 'https://nova.astrometry.net/api/submissions/16141301') {
      submissionChecks++;
      return Response.json(submissionChecks === 1
        ? { jobs: [null], processing_finished: 'None' }
        : { jobs: [123], job_calibrations: [[123, 456]] });
    }
    assert.equal(String(url), 'https://nova.astrometry.net/api/jobs/123');
    return Response.json({ status: 'success' });
  });
  assert.deepEqual(await h.service.pollForPlateSolvingResult('16141301'), { status: 'success', result, remoteJobId: '123' });
  assert.equal(submissionChecks, 2);
  assert.deepEqual(h.delays, [5000]);
});

const calibration = { ra: 0, dec: 0, pixscale: 3, radius: 4, orientation: 0 };
const completeResult = { calibration, annotations: [], machineTags: [] };
const base = 'https://nova.astrometry.net/api/';
function network(t: TestContext, queues: Record<string, (Response | Error | (() => Response))[]>, trace: string[] = []) {
  t.mock.method(globalThis, 'fetch', async (url) => {
    const key = String(url).replace(base, '');
    trace.push(`fetch:${key}`);
    const queue = queues[key];
    assert.ok(queue?.length, `Unexpected request: ${key}`);
    const item = queue.length === 1 ? queue[0] : queue.shift();
    if (item instanceof Error) throw item;
    return typeof item === 'function' ? item() : item!.clone();
  });
}
function resultQueues(id = '123') {
  return {
    [`jobs/${id}/calibration`]: [Response.json(calibration)],
    [`jobs/${id}/annotations`]: [Response.json([])],
    [`jobs/${id}/machine_tags/`]: [Response.json([])],
  };
}

for (const jobs of [undefined, null, [], [null]]) {
  for (const stamp of [null, 'None', 'null', '2020-01-01']) {
    it(`both callers wait for ${JSON.stringify(jobs)} jobs with ${stamp} timestamp`, async t => {
      const h = fixture(t);
      const trace: string[] = [];
      network(t, { 'submissions/16141301': [Response.json({ jobs, processing_finished: stamp })] }, trace);
      h.job.submittedAt = new Date('2000-01-01');
      assert.deepEqual(await h.service.checkJobStatus(14), { status: 'processing' });
      assert.deepEqual(await h.service.pollForPlateSolvingResult('16141301'), { status: 'processing', remoteJobId: null });
      assert.equal(trace.length, 721); // One worker check, then 720 direct submission attempts.
      assert.equal(h.delays.length, 720);
      assert.ok(h.delays.every(ms => ms === 5000));
      assert.deepEqual(h.writes, []);
    });
  }
}

for (const storedId of ['null', 'None', '', 'abc', '0', '-1', '1.5']) {
  it(`worker ignores invalid stored ID ${storedId}`, async t => {
    const h = fixture(t); h.job.astrometryJobId = storedId;
    network(t, { 'submissions/16141301': [Response.json({ jobs: [null] })] });
    assert.deepEqual(await h.service.checkJobStatus(14), { status: 'processing' });
    assert.deepEqual(h.writes, []);
  });
}

it('discovery persists before validated completion and preserves image enrichment', async t => {
  const h = fixture(t); h.job.imageId = 1;
  const trace: string[] = [];
  t.mock.method(storage, 'updatePlateSolvingJob', async (_id, patch) => {
    trace.push(`write:${patch.status ?? patch.astrometryJobId}`); h.writes.push(patch); Object.assign(h.job, patch); return h.job;
  });
  t.mock.method(storage, 'getAstroImage', async () => ({ id: 1, tags: [] }));
  let imagePatch;
  t.mock.method(storage, 'updateAstroImage', async (_id, patch) => { imagePatch = patch; });
  const { catalogService } = await import('./catalog');
  t.mock.method(catalogService, 'matchTargetFromTags', async () => []);
  t.mock.method(configService, 'getSidecarConfig', async () => ({ enabled: false }));
  setWsManager({ broadcast: (_event, data) => { trace.push(`event:${data.status}`); } } as never);
  t.after(() => setWsManager(null));
  network(t, { 'submissions/16141301': [Response.json({ jobs: [null, 123] })], 'jobs/123': [Response.json({ status: 'success' })], ...resultQueues() }, trace);
  assert.deepEqual(await h.service.checkJobStatus(14), { status: 'success', result: completeResult });
  assert.ok(trace.indexOf('fetch:jobs/123/calibration') < trace.indexOf('write:123'));
  assert.ok(trace.indexOf('write:123') < trace.indexOf('write:success'));
  assert.ok(trace.indexOf('write:success') < trace.indexOf('event:success'));
  assert.equal(imagePatch.pixelScale, 3); assert.equal(imagePatch.fieldOfView, "8.0'");
  assert.equal(imagePatch.plateSolved, true); assert.equal(imagePatch.astrometryJobId, '123');
});

it('selected ID survives empty jobs and calibrated different job overrides stale failure', async t => {
  const h = fixture(t); h.job.astrometryJobId = '123';
  network(t, { 'submissions/16141301': [Response.json({ jobs: [] }), Response.json({ jobs: [], error_message: 'stale', job_calibrations: [[456, 789]] })], 'jobs/123': [Response.json({ status: 'solving' })], ...resultQueues('456') });
  assert.equal((await h.service.checkJobStatus(14)).status, 'processing');
  assert.equal(h.job.astrometryJobId, '123');
  assert.equal((await h.service.checkJobStatus(14)).status, 'success');
  assert.equal(h.job.astrometryJobId, '456');
});

for (const mode of ['submission', 'job', '404']) {
  it(`both callers recognize explicit ${mode} failure`, async t => {
    const h = fixture(t);
    network(t, { 'submissions/16141301': [Response.json(mode === 'submission' ? { error_message: 'bad image' } : { jobs: [123] })], 'jobs/123': [mode === '404' ? new Response('', { status: 404 }) : Response.json({ status: 'failure' })] });
    assert.equal((await h.service.pollForPlateSolvingResult('16141301')).status, 'failed');
    assert.equal((await h.service.checkJobStatus(14)).status, 'failed');
    assert.equal(h.job.status, 'failed');
    assert.ok(!JSON.stringify(h.job.result).includes('30 days'));
  });
}

for (const bad of [new Error('offline'), new Response('', { status: 503 }), new Response('{'), Response.json({ jobs: 'invalid' })]) {
  it(`request errors preserve failed record and direct polling retries (${bad instanceof Error ? bad.message : bad.status})`, async t => {
    const h = fixture(t); h.job.status = 'failed'; h.job.result = { error: 'stale' };
    const before = structuredClone(h.job);
    network(t, { 'submissions/16141301': [bad, Response.json({ job_calibrations: [[123, 456]] })], ...resultQueues() });
    await assert.rejects(h.service.checkJobStatus(14, { resumeProcessing: true }));
    assert.deepEqual(h.job, before); assert.deepEqual(h.writes, []);
    assert.equal((await h.service.checkJobStatus(14)).status, 'success');
    network(t, { 'submissions/16141301': [bad, Response.json({ job_calibrations: [[123, 456]] })], ...resultQueues() });
    assert.deepEqual(await h.service.pollForPlateSolvingResult('16141301'), { status: 'success', result: completeResult, remoteJobId: '123' });
    assert.deepEqual(h.delays, [5000]);
  });
}

for (const bad of [new Response('', { status: 503 }), new Response('{'), Response.json({ error: 'unavailable' }), Response.json({ ...calibration, ra: '0' }), Response.json({ ...calibration, radius: null })]) {
  it(`invalid calibration leaves failed record untouched and can recover (${bad.status})`, async t => {
    const h = fixture(t); h.job.status = 'failed'; h.job.result = { error: 'stale' };
    const before = structuredClone(h.job);
    network(t, { 'submissions/16141301': [Response.json({ job_calibrations: [[123, 456]] })], ...resultQueues(), 'jobs/123/calibration': [bad, Response.json(calibration)] });
    await assert.rejects(h.service.checkJobStatus(14, { resumeProcessing: true }));
    assert.deepEqual(h.job, before); assert.deepEqual(h.writes, []);
    assert.equal((await h.service.checkJobStatus(14)).status, 'success');
    network(t, { 'submissions/16141301': [Response.json({ job_calibrations: [[123, 456]] })], ...resultQueues(), 'jobs/123/calibration': [bad, Response.json(calibration)] });
    assert.equal((await h.service.pollForPlateSolvingResult('16141301')).status, 'success');
    assert.deepEqual(h.delays, [5000]);
  });
}

it('mixed direct exhaustion retains ID and configured two-second delay', async t => {
  const h = fixture(t);
  t.mock.method(configService, 'getAstrometryConfig', async () => ({ pollInterval: 2 }));
  let checks = 0; let jobChecks = 0;
  t.mock.method(globalThis, 'fetch', async url => {
    if (String(url) === `${base}submissions/16141301`) {
      checks++; if (checks % 3 === 0) throw new Error('offline');
      return Response.json({ jobs: checks === 2 ? [123] : [] });
    }
    assert.equal(String(url), `${base}jobs/123`); jobChecks++; return Response.json({ status: 'solving' });
  });
  assert.deepEqual(await h.service.pollForPlateSolvingResult('16141301'), { status: 'processing', remoteJobId: '123' });
  assert.equal(checks, 720); assert.equal(jobChecks, 479);
  assert.equal(h.delays.length, 720); assert.ok(h.delays.every(ms => ms === 2000));
});

it('explicit recovery resumes same record for worker without submitting', async t => {
  const h = fixture(t); h.job.status = 'failed'; h.job.result = { error: 'stale' };
  t.mock.method(h.service, 'submitImageForPlateSolving', async () => { assert.fail('Must not submit'); });
  network(t, { 'submissions/16141301': [Response.json({ jobs: [] }), Response.json({ jobs: [] }), Response.json({ job_calibrations: [[123, 456]] })], ...resultQueues() });
  await h.service.checkJobStatus(14);
  assert.equal(h.job.status, 'failed'); assert.deepEqual(h.writes, []);
  await h.service.checkJobStatus(14, { resumeProcessing: true });
  assert.equal(h.job.status, 'processing'); assert.equal(h.job.result, null);
  t.mock.method(storage, 'getPlateSolvingJobs', async () => [h.job].filter(job => job.status === 'processing'));
  for (const job of await storage.getPlateSolvingJobs()) await h.service.checkJobStatus(job.id);
  assert.equal(h.job.id, 14); assert.equal(h.job.status, 'success');
});

it('invalid submission and result IDs never request upstream', async t => {
  const h = fixture(t); h.job.astrometrySubmissionId = 'null';
  network(t, {});
  await assert.rejects(h.service.checkJobStatus(14), /Invalid Astrometry submission ID/);
  for (const id of ['null', 'None', '-1', '0', '1.5']) await assert.rejects(h.service.fetchCompleteResult(id), /Invalid Astrometry job ID/);
  assert.deepEqual(h.writes, []);
});

it('optional annotations and tags remain fallbacks after valid calibration', async t => {
  const h = fixture(t);
  network(t, { 'jobs/123/calibration': [Response.json(calibration)], 'jobs/123/annotations': [new Error('offline')], 'jobs/123/machine_tags/': [new Error('offline')] });
  assert.deepEqual(await h.service.fetchCompleteResult('123'), completeResult);
});

for (const status of ['processing', 'success', 'failed']) {
  it(`workflow submits once and returns ${status} contract`, async t => {
    const h = fixture(t); let submissions = 0;
    t.mock.method(h.service, 'submitImageForPlateSolving', async () => { submissions++; return { submissionId: '16141301', jobId: 14 }; });
    network(t, { 'submissions/16141301': [Response.json(status === 'success' ? { job_calibrations: [[123, 456]] } : status === 'failed' ? { error_message: 'bad image' } : { jobs: [123] })], 'jobs/123': [Response.json({ status: 'solving' })], ...resultQueues() });
    if (status === 'failed') await assert.rejects(h.service.completePlateSolvingWorkflow({ id: 1 }), /bad image/);
    else assert.deepEqual(await h.service.completePlateSolvingWorkflow({ id: 1 }), status === 'success' ? { status, result: completeResult } : { status, jobId: 14, submissionId: '16141301', message: 'Still processing. Check status later.' });
    assert.equal(submissions, 1);
    if (status !== 'failed') assert.equal(h.job.astrometryJobId, '123');
    if (status === 'processing') assert.equal(h.job.status, 'processing');
  });
}

for (const bad of [new Error('offline'), new Response('', { status: 503 }), new Response('{'), Response.json({ status: 123 })]) {
  it(`job request errors do not persist discovered ID before recovery (${bad instanceof Error ? bad.message : bad.status})`, async t => {
    const h = fixture(t); h.job.status = 'failed'; h.job.result = { error: 'stale' };
    const before = structuredClone(h.job);
    network(t, { 'submissions/16141301': [Response.json({ jobs: [null, 123] })], 'jobs/123': [bad, Response.json({ status: 'success' })], ...resultQueues() });
    await assert.rejects(h.service.checkJobStatus(14, { resumeProcessing: true }));
    assert.deepEqual(h.job, before); assert.deepEqual(h.writes, []);
    assert.equal((await h.service.checkJobStatus(14)).status, 'success');
    network(t, { 'submissions/16141301': [Response.json({ jobs: [null, 123] })], 'jobs/123': [bad, Response.json({ status: 'success' })], ...resultQueues() });
    assert.deepEqual(await h.service.pollForPlateSolvingResult('16141301'), { status: 'success', result: completeResult, remoteJobId: '123' });
    assert.deepEqual(h.delays, [5000]);
  });
}

it('direct polling retains selected ID then prefers calibrated different job', async t => {
  const h = fixture(t); const ids: string[] = [];
  network(t, { 'submissions/16141301': [Response.json({ jobs: [null, 123] }), Response.json({ jobs: [] }), Response.json({ job_calibrations: [[456, 789]], error_message: 'stale' })], 'jobs/123': [Response.json({ status: 'solving' })], ...resultQueues('456') });
  assert.deepEqual(await h.service.pollForPlateSolvingResult('16141301', async id => { ids.push(id); }), { status: 'success', result: completeResult, remoteJobId: '456' });
  assert.deepEqual(ids, ['123', '456']); assert.deepEqual(h.delays, [5000, 5000]);
});
