import { it } from 'node:test';
import assert from 'node:assert/strict';
import { checkAstrometrySubmission, normalizeJobId, parseCalibration } from './astrometry-polling';

// A strict response queue catches invalid IDs, extra requests, and stale-job selection.
function upstream(submission: unknown, job?: unknown, submissionStatus = 200, jobStatus = 200) {
  const queue = new Map<string, Response[]>([
    ['https://nova.astrometry.net/api/submissions/10', [Response.json(submission, { status: submissionStatus })]],
    ['https://nova.astrometry.net/api/jobs/123', job === undefined ? [] : [Response.json(job, { status: jobStatus })]],
  ]);
  const calls: string[] = [];
  const fetchFn: typeof fetch = async input => {
    const url = String(input);
    calls.push(url);
    const response = queue.get(url)?.shift();
    assert.ok(response, `Unexpected fetch: ${url}`);
    return response;
  };
  return { fetchFn, calls };
}

for (const [name, submission] of [
  ['missing jobs', {}], ['null jobs', { jobs: null }], ['empty jobs', { jobs: [] }],
  ...[null, 'None', 'null', '2026-10-06 12:00:00'].map(timestamp =>
    [`null placeholders with ${timestamp}`, { jobs: [null], processing_finished: timestamp }] as const),
] as const) {
  it(`keeps ${name} processing`, async () => {
    const h = upstream(submission);
    assert.deepEqual(await checkAstrometrySubmission('10', null, h.fetchFn), { status: 'processing', jobId: null });
    assert.equal(h.calls.length, 1);
  });
}

for (const [name, jobs, selected] of [
  ['skips a null first slot', [null, 123], null],
  ['retains a selected job with empty jobs', [], '123'],
  ['normalizes the first usable job', ['null', 'None', 0, '00123'], 'null'],
] as const) {
  it(name, async () => {
    const h = upstream({ jobs }, { status: 'solving' });
    assert.deepEqual(await checkAstrometrySubmission('10', selected, h.fetchFn), { status: 'processing', jobId: '123' });
    assert.equal(h.calls.length, 2);
  });
}

it('uses the calibrated job rather than a stale selected failure or calibration ID', async () => {
  const h = upstream({ jobs: [null], job_calibrations: [[123, 456]], error_message: 'stale failure' });
  assert.deepEqual(await checkAstrometrySubmission('10', '999', h.fetchFn), { status: 'success', jobId: '123' });
  assert.equal(h.calls.length, 1);
});
it('skips malformed calibration pairs before the first usable pair', async () => {
  const h = upstream({ job_calibrations: [null, {}, [123], [123, 456, 789], [null, 456], [123, 'None'], ['00123', 456]] });
  assert.deepEqual(await checkAstrometrySubmission('10', null, h.fetchFn), { status: 'success', jobId: '123' });
});
it('malformed calibration pairs alone do not imply success', async () => {
  const h = upstream({ job_calibrations: [[null, 456], [123, 0]] });
  assert.deepEqual(await checkAstrometrySubmission('10', null, h.fetchFn), { status: 'processing', jobId: null });
});
it('reports an explicit submission failure without querying a job', async () => {
  const h = upstream({ jobs: [123], error_message: 'Unable to process submission' });
  assert.deepEqual(await checkAstrometrySubmission('10', null, h.fetchFn), { status: 'failed', jobId: '123', error: 'Unable to process submission' });
  assert.equal(h.calls.length, 1);
});
it('ignores blank submission error messages', async () => {
  const h = upstream({ error_message: '  ' });
  assert.deepEqual(await checkAstrometrySubmission('10', null, h.fetchFn), { status: 'processing', jobId: null });
});
for (const status of ['success', 'failure', 'mystery', undefined]) {
  it(`classifies selected job status ${status}`, async () => {
    const h = upstream({ jobs: [123] }, status === undefined ? {} : { status });
    const outcome = await checkAstrometrySubmission('10', null, h.fetchFn);
    assert.equal(outcome.status, status === 'success' ? 'success' : status === 'failure' ? 'failed' : 'processing');
    assert.equal(outcome.jobId, '123');
    if (outcome.status === 'failed') assert.match(outcome.error, /could not solve/i);
  });
}
it('classifies a valid selected job 404 as unavailable', async () => {
  const h = upstream({ jobs: [123] }, {}, 200, 404);
  const outcome = await checkAstrometrySubmission('10', null, h.fetchFn);
  assert.equal(outcome.status, 'failed');
  if (outcome.status === 'failed') {
    assert.match(outcome.error, /not found|unavailable/i);
    assert.doesNotMatch(outcome.error, /30 days|expired/i);
  }
});
for (const status of [400, 401, 404, 429, 500, 503]) {
  it(`throws a check error for submission HTTP ${status}`, async () => {
    const h = upstream({}, undefined, status);
    await assert.rejects(checkAstrometrySubmission('10', null, h.fetchFn), new RegExp(`Submission 10.*${status}`));
  });
  if (status !== 404) it(`throws a check error for job HTTP ${status}`, async () => {
    const h = upstream({ jobs: [123] }, {}, 200, status);
    await assert.rejects(checkAstrometrySubmission('10', null, h.fetchFn), new RegExp(`Job 123.*${status}`));
  });
}
for (const [context, body] of [
  ['Submission 10', { status: 'error' }], ['Submission 10', null], ['Submission 10', 123],
  ['Submission 10', { jobs: {} }], ['Submission 10', { job_calibrations: 'bad' }],
  ['Submission 10', { error_message: 123 }], ['Submission 10', { status: 123 }],
  ['Job 123', { status: 'error' }], ['Job 123', null], ['Job 123', 'bad'], ['Job 123', { status: 123 }],
] as const) {
  it(`rejects malformed ${context} response ${JSON.stringify(body)}`, async () => {
    const h = context === 'Submission 10' ? upstream(body) : upstream({ jobs: [123] }, body);
    await assert.rejects(checkAstrometrySubmission('10', null, h.fetchFn), new RegExp(context));
  });
}
for (const context of ['Submission 10', 'Job 123']) {
  it(`retains JSON parse cause with ${context} diagnostics`, async () => {
    const fetchFn: typeof fetch = async input => String(input).includes('/submissions/') && context === 'Job 123'
      ? Response.json({ jobs: [123] }) : new Response('{');
    await assert.rejects(checkAstrometrySubmission('10', null, fetchFn), error => {
      assert.ok(error instanceof Error);
      assert.match(error.message, new RegExp(context));
      assert.ok(error.cause instanceof SyntaxError);
      return true;
    });
  });
}
it('propagates network errors without a terminal observation', async () => {
  const error = new Error('network offline');
  const fetchFn: typeof fetch = async () => { throw error; };
  await assert.rejects(checkAstrometrySubmission('10', null, fetchFn), error);
});
it('rejects unusable remote IDs', () => {
  for (const id of [null, undefined, '', 'null', 'None', 0, -1, 1.5, true, {}, 'abc', Number.MAX_SAFE_INTEGER + 1, '1e3', ' 123 ']) {
    assert.equal(normalizeJobId(id), null);
  }
  for (const id of [123, '123', '00123']) assert.equal(normalizeJobId(id), '123');
});
const calibration = { ra: 0, dec: 0, pixscale: 1, radius: 1, orientation: 0 };
it('accepts finite zero calibration fields and optional dimensions', () => {
  assert.deepEqual(parseCalibration(calibration), calibration);
  const complete = { ...calibration, width_arcsec: 0, height_arcsec: 20, parity: -1 };
  assert.deepEqual(parseCalibration(complete), complete);
});
for (const field of ['ra', 'dec', 'pixscale', 'radius', 'orientation']) {
  for (const value of [undefined, '0', NaN, Infinity, -Infinity]) {
    it(`rejects calibration ${field}=${value}`, () => {
      assert.throws(() => parseCalibration({ ...calibration, [field]: value }));
    });
  }
}
it('rejects calibration error envelopes', () => {
  assert.throws(() => parseCalibration({ status: 'error', errormessage: 'Unavailable' }));
});
for (const field of ['width_arcsec', 'height_arcsec', 'parity']) {
  it(`rejects invalid optional calibration ${field}`, () => {
    assert.throws(() => parseCalibration({ ...calibration, [field]: Infinity }));
  });
}
